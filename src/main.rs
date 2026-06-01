// Before you begin reading, let me apologize beforehand for the slop code, although I have written in Rust before,
// this is my first time creating something this large with it.

const CONFIG_FILENAME: &str = ".roflow.json";

fn scan_folder(folder: &std::path::Path) -> Vec<std::path::PathBuf> {
    // From my understanding, vecs are growable arrays?
    let mut files = Vec::new();

    // Gives list of children of the passed directory
    let entries = std::fs::read_dir(folder).unwrap();
    let mut init_files = Vec::new();

    for entry in entries {
        // Must unwrap since entry is a result I think probably maybe?
        let entry = entry.unwrap();

        // Grab the actual path
        let path = entry.path();

        if path.is_file() && is_init_file(&path) {
            init_files.push(path);
            break;
        }
    }

    if init_files.len() > 0 {
        return init_files;
    }

    let entries = std::fs::read_dir(folder).unwrap();

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        // Recursion baby, we NEVER working at NASA!
        if path.is_dir() {
            files.extend(scan_folder(&path));
        } else if is_valid_source(&path) {
            files.push(path);
        }
    }

    files
}

fn is_valid_source(file: &std::path::Path) -> bool {
    let file_name = file.file_name().unwrap().to_str().unwrap().to_lowercase();

    file_name.ends_with(".luau")
        || file_name.ends_with(".lua")
        || file_name.ends_with(".rbxm")
        || file_name.ends_with(".rbxmx")
}

fn is_init_file(file: &std::path::Path) -> bool {
    if !is_valid_source(file) {
        return false;
    }

    let file_stem = file.file_stem().unwrap().to_str().unwrap().to_lowercase();

    file_stem == "init"
        || file_stem == "index"
        || file_stem.starts_with("init.")
        || file_stem.starts_with("index.")
        || file_stem.starts_with("init-")
        || file_stem.starts_with("index-")
        || file_stem.starts_with("init_")
        || file_stem.starts_with("index_")
}

fn get_aliases(json: &serde_json::Value) -> std::collections::HashMap<String, String> {
    let mut aliases = std::collections::HashMap::new();

    if !json["aliases"].is_object() {
        return aliases;
    }

    for (alias, route) in json["aliases"].as_object().unwrap() {
        let route = route.as_str().unwrap_or("");
        if route == "" {
            continue;
        }

        aliases.insert(alias.clone(), route.to_string());
    }

    aliases
}

fn route_from_word(word: &str, aliases: &std::collections::HashMap<String, String>) -> String {
    let word = word.to_lowercase();

    for (alias, route) in aliases {
        if alias.to_lowercase() == word {
            return route.clone();
        }
    }

    match word.as_str() {
        "server" => "ServerScriptService".to_string(),
        "client" => "ReplicatedStorage/Client".to_string(),
        "shared" => "ReplicatedStorage/Shared".to_string(),
        "player" => "StarterPlayer/StarterPlayerScripts".to_string(),
        "character" => "StarterPlayer/StarterCharacterScripts".to_string(),
        "serverscriptservice" => "ServerScriptService".to_string(),
        "replicatedstorage" => "ReplicatedStorage".to_string(),
        "replicatedfirst" => "ReplicatedFirst".to_string(),
        "serverstorage" => "ServerStorage".to_string(),
        "startergui" => "StarterGui".to_string(),
        "starterpack" => "StarterPack".to_string(),
        "starterplayerscripts" => "StarterPlayer/StarterPlayerScripts".to_string(),
        "startercharacterscripts" => "StarterPlayer/StarterCharacterScripts".to_string(),
        _ => "".to_string(),
    }
}

fn get_suffix_route(
    file_stem: &str,
    aliases: &std::collections::HashMap<String, String>,
) -> (String, usize) {
    let lower = file_stem.to_lowercase();

    for separator in [".", "-", "_"] {
        let index = lower.rfind(separator).unwrap_or(usize::MAX);
        if index == usize::MAX {
            continue;
        }
        let suffix = &file_stem[index + 1..];
        let route = route_from_word(suffix, aliases);

        if route != "" {
            return (route, file_stem.len() - index);
        }
    }

    let mut suffixes = vec![
        "Server".to_string(),
        "Client".to_string(),
        "Shared".to_string(),
        "Player".to_string(),
        "Character".to_string(),
        "ServerScriptService".to_string(),
        "ReplicatedStorage".to_string(),
        "ReplicatedFirst".to_string(),
        "ServerStorage".to_string(),
        "StarterGui".to_string(),
        "StarterPack".to_string(),
        "StarterPlayerScripts".to_string(),
        "StarterCharacterScripts".to_string(),
    ];

    for alias in aliases.keys() {
        suffixes.push(alias.clone());
    }

    for suffix in suffixes {
        if file_stem.ends_with(&suffix) && file_stem.len() > suffix.len() {
            let route = route_from_word(&suffix, aliases);

            if route != "" {
                return (route, suffix.len());
            }
        }
    }

    ("".to_string(), 0)
}

fn route_file(
    file: &std::path::Path,
    source: &std::path::Path,
    build_folder: &str,
    aliases: &std::collections::HashMap<String, String>,
) -> (String, Vec<String>, String, String) {
    let relative_path = file.strip_prefix(source).unwrap();
    let mut route = "ReplicatedStorage".to_string();
    let mut has_folder_route = false;
    let mut last_route_folder = "".to_string();
    let mut folders = Vec::new();

    let parent = relative_path.parent().unwrap_or(std::path::Path::new(""));
    if parent.as_os_str() != "" {
        for part in parent {
            let part = part.to_str().unwrap().to_string();
            let folder_route = route_from_word(&part, aliases);

            if folder_route != "" {
                route = folder_route;
                has_folder_route = true;
                last_route_folder = part;
            } else {
                folders.push(part);
            }
        }
    }

    let file_stem = file.file_stem().unwrap().to_str().unwrap();
    let mut name = file_stem.to_string();
    let (suffix_route, suffix_length) = get_suffix_route(file_stem, aliases);

    if suffix_route != "" {
        if !has_folder_route {
            route = suffix_route;
        }

        name = file_stem[..file_stem.len() - suffix_length].to_string();
    }

    let mut project_relative_path = relative_path.to_string_lossy().replace('\\', "/");

    if is_init_file(file) {
        let parent = relative_path.parent().unwrap();
        project_relative_path = parent.to_string_lossy().replace('\\', "/");

        if folders.len() > 0 {
            name = folders.pop().unwrap();
        } else if last_route_folder != "" {
            name = last_route_folder;
        } else {
            name = "source".to_string();
        }
    }

    let project_path = if project_relative_path == "" {
        build_folder.to_string()
    } else {
        format!("{}/{}", build_folder, project_relative_path).replace('\\', "/")
    };

    (route, folders, name, project_path)
}

fn insert_file(
    project: &mut serde_json::Value,
    route: &str,
    folders: &[String],
    name: &str,
    path: &str,
) {
    let mut current = &mut project["tree"];

    for part in route.split('/') {
        if current[part].is_null() {
            current[part] = serde_json::json!({
                "$className": get_class_name(part)
            });
        }

        current = &mut current[part];
    }

    for folder in folders {
        if current[folder].is_null() {
            current[folder] = serde_json::json!({
                "$className": "Folder"
            });
        }

        current = &mut current[folder];
    }

    current[name] = serde_json::json!({
        "$path": path
    });
}

fn get_class_name(part: &str) -> &str {
    match part {
        "Client" | "Shared" => "Folder",
        _ => part,
    }
}

fn create_project() -> serde_json::Value {
    let mut project = serde_json::json!({
        "emitLegacyScripts": false,
        "name": "roflow-project",
        "tree": {
            "$className": "DataModel"
        }
    });

    project["tree"]["ReplicatedStorage"] = serde_json::json!({
        "$className": "ReplicatedStorage",
        "Client": {
            "$className": "Folder"
        },
        "Shared": {
            "$className": "Folder"
        }
    });

    project["tree"]["ServerScriptService"] = serde_json::json!({
        "$className": "ServerScriptService"
    });

    project["tree"]["ServerStorage"] = serde_json::json!({
        "$className": "ServerStorage"
    });

    project["tree"]["ReplicatedFirst"] = serde_json::json!({
        "$className": "ReplicatedFirst"
    });

    project["tree"]["StarterGui"] = serde_json::json!({
        "$className": "StarterGui"
    });

    project["tree"]["StarterPack"] = serde_json::json!({
        "$className": "StarterPack"
    });

    project["tree"]["StarterPlayer"] = serde_json::json!({
        "$className": "StarterPlayer",
        "StarterPlayerScripts": {
            "$className": "StarterPlayerScripts"
        },
        "StarterCharacterScripts": {
            "$className": "StarterCharacterScripts"
        }
    });

    project
}

fn print_help() {
    println!("roflow");
    println!("commands:");
    println!("  init                  create .roflow.json");
    println!("  build                 create default.project.json");
    println!("  serve [--include-rojo] watch and rebuild");
    println!("  install               install roflow locally");
}

fn default_config() -> &'static str {
    r#"{
	"source": "src",
	"build": "src",
	"output": "default.project.json",
	"includeRojo": false,
	"aliases": {
		"Controller": "ReplicatedStorage/Client",
		"Service": "ServerScriptService"
	}
}"#
}

fn run_init() {
    // Self explanatory but what the heck path::Path, and why am I calling new to check if it exists?
    // UPDATE, I learned what it is and now I feel dumb.... Although I still think that's bad design!!
    if std::path::Path::new(CONFIG_FILENAME).exists() {
        println!("A roflow configuration already exists u silly guy!");
        return;
    }

    std::fs::write(CONFIG_FILENAME, default_config()).unwrap();

    println!("created .roflow.json uwu");
}

fn run_install() {
    let current_exe = std::env::current_exe().unwrap();
    let home = get_home_folder();
    let install_folder = std::path::Path::new(&home).join(".roflow");
    let install_path = install_folder.join(get_installed_exe_name());

    std::fs::create_dir_all(&install_folder).unwrap();
    std::fs::copy(&current_exe, &install_path).unwrap();

    println!("installed roflow to {}", install_path.display());

    add_install_folder_to_path(&install_folder);
}

fn get_home_folder() -> String {
    let home = std::env::var("HOME").unwrap_or("".to_string());
    if home != "" {
        return home;
    }

    std::env::var("USERPROFILE").unwrap_or(".".to_string())
}

fn get_installed_exe_name() -> &'static str {
    if cfg!(windows) {
        return "roflow.exe";
    }

    "roflow"
}

fn add_install_folder_to_path(install_folder: &std::path::Path) {
    if cfg!(windows) {
        add_install_folder_to_windows_path(install_folder);
        return;
    }

    print_unix_path_instructions(install_folder);
}

fn add_install_folder_to_windows_path(install_folder: &std::path::Path) {
    let install_folder = install_folder.to_str().unwrap();
    let user_path_output = std::process::Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("[Environment]::GetEnvironmentVariable('Path', 'User')")
        .output();

    if user_path_output.is_err() {
        println!("failed to read user PATH automatically");
        println!("add this folder to PATH manually:");
        println!("{install_folder}");
        return;
    }

    let user_path_output = user_path_output.unwrap();
    let user_path = String::from_utf8_lossy(&user_path_output.stdout)
        .trim()
        .to_string();
    let mut already_in_path = false;

    for path in user_path.split(';') {
        if path.to_lowercase() == install_folder.to_lowercase() {
            already_in_path = true;
        }
    }

    if already_in_path {
        println!("roflow install folder is already in PATH");
        return;
    }

    let new_path = if user_path == "" {
        install_folder.to_string()
    } else {
        format!("{};{}", user_path, install_folder)
    };

    let result = std::process::Command::new("powershell")
        .env("ROFLOW_NEW_PATH", &new_path)
        .arg("-NoProfile")
        .arg("-Command")
        .arg("[Environment]::SetEnvironmentVariable('Path', $env:ROFLOW_NEW_PATH, 'User')")
        .status();

    if result.is_err() {
        println!("failed to update PATH automatically");
        println!("add this folder to PATH manually:");
        println!("{install_folder}");
        return;
    }

    let result = result.unwrap();
    if !result.success() {
        println!("failed to update PATH automatically");
        println!("add this folder to PATH manually:");
        println!("{install_folder}");
        return;
    }

    println!("added roflow install folder to PATH");
    println!("restart your terminal before running roflow globally");
}

fn print_unix_path_instructions(install_folder: &std::path::Path) {
    let install_folder = install_folder.to_str().unwrap();
    let current_path = std::env::var("PATH").unwrap_or("".to_string());
    let mut already_in_path = false;

    for path in current_path.split(':') {
        if path == install_folder {
            already_in_path = true;
        }
    }

    if already_in_path {
        println!("roflow install folder is already in PATH");
        return;
    }

    println!("add this folder to PATH:");
    println!("{install_folder}");
    println!("for bash/zsh, add this to your shell config:");
    println!("export PATH=\"{install_folder}:$PATH\"");
}

fn load_config() -> serde_json::Value {
    // Grab the file contents
    let config = std::fs::read_to_string(CONFIG_FILENAME).unwrap();

    // Parse the config text into JSON data
    serde_json::from_str(&config).unwrap()
}

fn run_build() -> String {
    if !std::path::Path::new(CONFIG_FILENAME).exists() {
        println!("You must initialize roflow with `roflow init` before building");
        return "default.project.json".to_string();
    }

    let json = load_config();

    // Grabs a key from json thats named `source` and we check the value against the path!
    let source = json["source"].as_str().unwrap_or("src");
    let build_folder = json["build"].as_str().unwrap_or(source);
    let output_file = json["output"].as_str().unwrap_or("default.project.json");
    let aliases = get_aliases(&json);
    let source_path = std::path::Path::new(source);

    if !source_path.exists() {
        println!("source folder does NOT exist");
        return output_file.to_string();
    }

    let files = scan_folder(source_path);
    let mut project = create_project();

    // We can iterate over the source directory,
    // and build the sourcemap from there once the file hierarchy is checked.
    for file in files {
        let (route, folders, name, path) = route_file(&file, source_path, build_folder, &aliases);

        insert_file(&mut project, &route, &folders, &name, &path);
    }

    let output = serde_json::to_string_pretty(&project).unwrap();
    std::fs::write(output_file, output).unwrap();

    println!("created {output_file}");
    output_file.to_string()
}

fn run_serve(include_rojo_arg: bool) {
    let json = load_config();
    let source = json["source"].as_str().unwrap_or("src").to_string();
    let include_rojo = include_rojo_arg || json["includeRojo"].as_bool().unwrap_or(false);
    let project_file = run_build();
    let mut rojo_processes = Vec::new();

    if include_rojo {
        rojo_processes = start_rojo(&project_file);
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher: notify::RecommendedWatcher = notify::recommended_watcher(move |result| {
        tx.send(result).unwrap();
    })
    .unwrap();

    notify::Watcher::watch(
        &mut watcher,
        std::path::Path::new(&source),
        notify::RecursiveMode::Recursive,
    )
    .unwrap();

    println!("watching {source}");

    if !include_rojo {
        println!("rojo serving is disabled, pass --include-rojo to run rojo serve too");
    }

    for result in rx {
        match result {
            Ok(_) => {
                run_build();
            }
            Err(error) => println!("watch error: {error}"),
        }
    }

    for child in rojo_processes.iter_mut() {
        child.kill().ok();
    }
}

fn start_rojo(project_file: &str) -> Vec<std::process::Child> {
    let mut processes = Vec::new();
    let candidates = [
        "rojo",
        "./rojo",
        "./rojo.exe",
        "./bin/rojo",
        "./bin/rojo.exe",
    ];

    for candidate in candidates {
        let child = std::process::Command::new(candidate)
            .arg("serve")
            .arg(project_file)
            .spawn();

        if child.is_ok() {
            println!("started rojo with {candidate}");
            processes.push(child.unwrap());
            return processes;
        }
    }

    println!("could not find rojo, continuing without it");
    processes
}

fn main() {
    // Grab the arguments passed through the command
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 1 {
        print_help();
        return;
    }

    // Grab the first command from args
    let command = &args[1];
    let include_rojo = args.iter().any(|arg| arg == "--include-rojo");

    match command.as_str() {
        "init" | "--init" => run_init(),
        "build" | "--build" => {
            run_build();
        }
        "serve" | "--serve" => run_serve(include_rojo),
        "install" | "--install" => run_install(),
        _ => println!("unknown command: {command}"),
    }
}
