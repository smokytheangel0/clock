use super::*;

fn download(data: &String) -> BridgeResult {

}

fn upload(data: &String) -> BridgeResult {

}

fn copy_from_usb(data: &String) -> BridgeResult {

}

fn copy_to_usb(data: &String) -> BridgeResult {

}

//rules for db
// must use username from projects.txt
// should be tolerant to missing fields
// or ask for credentials
// should pull down on first start for the day
// should push up each time the db is closed
// should also have a sync key
// should error gracefully into results
// might make it so we copy to usb in the event that internet is not working
// and usb should error gracefully with "please insert a usb drive to sync locally"
//client_id = 443395704515-u7hce21564ts1fupurbi5u0ndblvtfst.apps.googleusercontent.com
//client_secret = 9gWsF054pfXKjUjdBR-T0A-j
//we also have the json file in the src directory here
//clock.db id = 1c4Kwl9FnES4GZ1yX3fPqG8m0CYojgus7



fn binary_switch(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        state: i64
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };

    if arguments.state == 0 {
        arguments.state += 1;
    } else {
        arguments.state -= 1;
    }

    BridgeResult::ok(arguments.state)
}

fn emotify(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };

    let mut app = arguments.app;


    let mut mood = "".to_string();
    if app.mood == 0 {
        mood = "😩".to_string();
    } else {
        mood = "😁".to_string();
    }
    let mut learning = "".to_string();
    if app.learning == 0 {
        learning = "➖".to_string();
    } else {
        learning = "➕".to_string();
    }
    
    BridgeResult{
        result: "Ok()".to_string(),
        data: vec![mood, learning]
    }
}

fn storage(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        table: String,
        data: Vec<Value>,
        path: String
    }

    let arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };


    let storage = match Connection::open(format!("{}/clock.db", arguments.path)) {
        Ok(storage) => storage,
        Err(err) => return BridgeResult::err("failed to open connection to db: {:?}", err)
    };

    //this can definitely be optimized to return a single result instead of the entire table
    let statement = match storage.prepare(&format!("SELECT * FROM {}", arguments.table)) {
        Ok(statement) => statement,
        Err(err) => return BridgeResult::err("failed to prepare column identifying statement: {:?}", err)
    };

    let columns: Vec<&str> = statement.column_names();

    let storage = match Connection::open(format!("{}/clock.db", arguments.path)) {
        Ok(storage) => storage,
        Err(err) => return BridgeResult::err("failed to open connection to db: {:?}", err)
    };

    let mut column_values: Vec<SqlValue> = vec![];
    for value in arguments.data {
        if value.is_string() {
            match value.as_str() {
                Some(string) => column_values.push(SqlValue::Text(string.to_owned())),
                None => return BridgeResult::err("impossible input value: {:?}", value)
            }
        } else if value.is_i64() {
            match value.as_i64() {
                Some(int) => column_values.push(SqlValue::Integer(int)),
                None => return BridgeResult::err("impossible input value: {:?}", value)
            }
        } else if value.is_f64() {
            match value.as_f64() {
                Some(float) => column_values.push(SqlValue::Real(float)),
                None => return BridgeResult::err("impossible input value: {:?}", value)
            }
        } else {
            return BridgeResult::err("the only types accepted for storage right now are String, Floating Point and Integer", 1)
        }
    }
    let local: DateTime<Local> = Local::now();
    column_values.push(SqlValue::Integer(local.timestamp()));

    //create question mark string according to how many values are present
    let mut interro_string = "".to_owned();
    for number in 0..columns.len() {
        if number != 0 {
            interro_string = format!("{}, ?{}", interro_string, number + 1).to_owned();
        } else {
            interro_string = "?1".to_owned();
        }
    }

    let table_statement = format!("INSERT INTO {} ({}) VALUES ({})", arguments.table, columns.join(","), interro_string);

    let mut statement = match storage.prepare(&table_statement){
        Ok(statement) => statement,
        Err(err) => return BridgeResult::err("failed to prepare the insertion statment: {:?}", err)
    };

    match statement.execute(&column_values) {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to write to db: {:?}", err)
    };

    match statement.finalize() {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to close the db: {:?}", err)
    };

    let table_statement = format!("INSERT INTO total ({}) VALUES ({})", columns.join(","), interro_string);

    let mut statement = match storage.prepare(&table_statement){
        Ok(statement) => statement,
        Err(err) => return BridgeResult::err("failed to prepare the insertion statment: {:?}", err)
    };

    match statement.execute(column_values) {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to write to db: {:?}", err)
    };

    match statement.finalize() {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to close the db: {:?}", err)
    };


    BridgeResult::ok("successfully wrote to db")
}

fn store(app: &mut App) -> BridgeResult {
    let local: DateTime<Local> = Local::now();

    let output = get_commit(&app.projects, &app.selected);
    let mut commit_id = "None".to_string();
    if output.result != "Ok()" {
        app.result.push(output.data[0].clone()); 
    } else {
        commit_id = output.data[0].clone();
    }
    
    let (output, test_map) = count_tests(&app.projects, &app.selected);

    if output.result != "Ok()" {
        return output
    } else {
        app.tests = match output.data[0].parse::<i64>() {
            Ok(test_count) => test_count,
            Err(err) => return BridgeResult::err("failed to convert test_count to integer", err)
        }
    }

    let doses = app.water + app.caffeine + app.nicotine + app.thc + app.food + app.meds;

    let to_store = ToStore {
        table: app.selected.clone(),
        data: vec![
                    app.selected.clone(), 
                    app.state.clone(), 
                    app.comment.clone(),  
                    commit_id, 
                    app.tests.to_string(), 
                    app.mood.to_string(), 
                    app.learning.to_string(), 
                    doses.to_string(), 
                    app.water.to_string(), 
                    app.caffeine.to_string(), 
                    app.nicotine.to_string(), 
                    app.thc.to_string(), 
                    app.food.to_string(), 
                    app.alcohol.to_string(), 
                    app.hungover.to_string(), 
                    app.meds.to_string(), 
                    local.hour().to_string(), 
                    local.minute().to_string(), 
                    local.second().to_string(), 
                    local.day().to_string(), 
                    local.month().to_string(), 
                    local.year().to_string()
                ],
        path: match env::current_dir(){
            Ok(dir) => dir,
            Err(err) => return BridgeResult::err("failed to get current dir in store function: {}", err)
        }.display().to_string()
    };

    let store_string = match serde_json::to_string(&to_store){
        Ok(string) => string,
        Err(err) => return BridgeResult::err("failed to encode insertion string in store: {}", err)
    };

    let output = storage(&store_string);

    if output.result == "Ok()" {
        let app_string = match serde_json::to_string(&app) {
            Ok(app_string) => app_string,
            Err(err) => return BridgeResult::err("failed to encode app struct to string in init", 0)
        };

        return BridgeResult::ok(app_string)
    }
    output   

}

fn start(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };

    let mut app = arguments.app;
    app.comment = format!("started clock");
    app.state = format!("play");

    let output = store(&mut app);

    output

}

fn play(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };
    let mut app = arguments.app;
    app.comment = format!("resumed from idle");
    app.state = format!("play");

    let output = store(&mut app);

    output
}

fn exit(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };
    let mut app = arguments.app;
    app.comment = format!("exited clock");
    app.state = format!("pause");

    let output = store(&mut app);

    output

}

fn pause(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };
    let mut app = arguments.app;

    app.comment = format!("paused for idle");
    app.state = format!("pause");

    //pass to new store function

    let output = store(&mut app);
    
    output
}

fn store_input(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };
    let mut app = arguments.app;

    let output = store(&mut app);
    
    output

}

fn change_project(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };

    let mut app = arguments.app;
    let mut select_list = vec![];
    for project in app.projects.map.keys() {
        select_list.push(project.clone());
    }
    //its weird that I should have to do this with an index map
    select_list.sort();


    let mut project_index = match select_list.iter().position(|s| s == &app.selected) {
        Some(project_index) => project_index,
        None => return BridgeResult::err("failed to get project index", 1)
    };

    if project_index == select_list.len() - 1 {
        project_index = 0;
    } else {
        project_index += 1;
    }

    app.comment = format!("switched to {}", select_list[project_index]);
    app.state = format!("pause");

    let output = store(&mut app);
    
    let mut last_index: usize = 0;

    if project_index != 0 {
        last_index = project_index - 1;
    } else {
        last_index = select_list.len() - 1;
    }

    app.selected = select_list[project_index].clone();

    app.comment = format!("switched from {}", select_list[last_index]);
    app.state = format!("play");

    let output = store(&mut app);
    output
}

fn idle_time(data: &String) -> BridgeResult {
    //ioreg -c IOHIDSystem | awk '/HIDIdleTime/ {print $NF/1000000000; exit}' this gives us the idle time in seconds i think
    #[derive(Deserialize)]
    struct Arguments {
        app: App
    }

    let mut arguments: Arguments = match serde_json::from_str(&data) {
        Ok(data) => data,
        Err(err) => return BridgeResult::err("failed to parse arguments, {}", err)
    };

    let mut app = arguments.app;

    if cfg!(target_os="macos") {
        /*
        let mut cmd_ioreg = match Command::new("ioreg").arg("-c").arg("IOHIDSystem").stdout(Stdio::piped()).spawn() {
            Ok(cmd) => cmd,
            Err(err) => return BridgeResult::err("failed to run ioreg: {}", err)
        };

        let mut cmd_awk = match Command::new("awk").arg("/HIDIdleTime/ {print $NF/1000000000; exit}").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn() {
            Ok(cmd) => cmd,
            Err(err) => return BridgeResult::err("failed to run awk command: {}", err)
        };

        if let Some(ref mut stdout) = cmd_ioreg.stdout {
            if let Some(ref mut stdin) = cmd_awk.stdin {
                let mut buf: Vec<u8> = Vec::new();
                match stdout.read_to_end(&mut buf) {
                    Ok(_) => (),
                    Err(err) => return BridgeResult::err("failed to read stdout: {}", err)
                };
                //fails here
                match stdin.write_all(&buf){
                    Ok(_) => (),
                    Err(err) => return BridgeResult::err("failed to write to stdin: {}", err)
                };
            };
        };

        let output = match cmd_awk.wait_with_output(){
            Ok(cmd) => cmd,
            Err(err) => return BridgeResult::err("failed to wait on awk to finish: {}", err)
        };
        */
        let output = Command::new("./mac_idle.sh").output().expect("failed to run mac_idle");
        let stdout = match str::from_utf8(&output.stdout){
            Ok(output) => output,
            Err(err) => return BridgeResult::err("failed to convert u8 output to utf8: {}", err)
        };
        let float = match stdout.trim().parse::<f64>() {
            Ok(float) => float,
            Err(err) => return BridgeResult::err("failed to parse f64 from command output: {}", err)
        };

        app.idle = float;

    } else if cfg!(target_os="linux") {
        let output = match Command::new("xprintidle").output(){
            Ok(cmd) => cmd,
            Err(err) => return BridgeResult::err("failed to run xprintidle command: {}", err)
        };
        let stdout = match str::from_utf8(&output.stdout){
            Ok(output) => output,
            Err(err) => return BridgeResult::err("failed to convert u8 output to utf8: {}", err)
        };
        let float = match stdout.trim().parse::<f64>() {
            Ok(float) => float,
            Err(err) => return BridgeResult::err("failed to parse f64 from command output: {}", err)    
        };
        let float = float/1000.0;
        app.idle = float;
    } else if cfg!(target_os="windows") {
        return BridgeResult::err("windows idle is not supported yet", 1)
    }
    let app_string = match serde_json::to_string(&app) {
        Ok(app_string) => app_string,
        Err(err) => return BridgeResult::err("failed to encode app struct to string in init", 1)
    };

    return BridgeResult::ok(
        app_string
    );
}

fn get_commit(project: &Projects, selected: &String) -> BridgeResult {
    //run git rev-parse HEAD in the project folder, collect the output string
    let project_dir = &project.map[selected];
    
    let exec_dir = match env::current_dir(){
        Ok(dir) => dir,
        Err(err) => return BridgeResult::err("failed to get exec dir: {}", err)
    };
    
    match env::set_current_dir(project_dir){
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to set project dir for commit history: {}", err)
    };

    let output = match Command::new("git").arg("rev-parse").arg("HEAD").output(){
        Ok(output) => output,
        Err(err) => return BridgeResult::err("failed to run git: {}", err)
    };

    let stdout = match str::from_utf8(&output.stdout){
        Ok(stdout) => stdout,
        Err(err) => return BridgeResult::err("failed to run git: {}", err)
    };

    match env::set_current_dir(exec_dir){
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to reset dir to exec dir: {}", err)
    };
    
    if !output.status.success() {
        return BridgeResult::err("there is no repo for this project yet", 1)
    } else {
        return BridgeResult::ok(
            stdout
        )
    }
}

fn count_tests(project: &Projects, selected: &String) -> (BridgeResult, HashMap<String, Vec<String>>) {
    let project_dir = &project.map[selected];

    let mut folders_to_search: Vec<PathBuf> = Vec::new();
    let mut test_count = 0;
    let mut type_of_file = String::new();

    let mut test_map = HashMap::new();
    let mut key_map = HashMap::new();


    let mut debug_print = String::new();


    for entry in match fs::read_dir(project_dir){
        Ok(entry) => entry,
        Err(err) => return (BridgeResult::err("failed to get list of files in project dir", err), test_map)
    } {
        let entry = match entry{
            Ok(entry) => entry,
            Err(err) => return (BridgeResult::err("failed to unwrap entry", err), test_map)
        };

        if entry.path().is_dir() {
            let name = match entry.file_name().into_string(){
                Ok(name) => name,
                Err(err) => return (BridgeResult::err("failed to convert osstring to String", err), test_map)
            };
            if name.contains("lib")
                || name.contains("src")
                || name.contains("rust")
                || name.contains("test")
                || name.contains("app_test")
            {
                if name.contains("rust") {
                    //panics right here
                    for entry in match fs::read_dir(entry.path()){
                        Ok(entry) => entry,
                        Err(err) => return (BridgeResult::err("failed to get a list of folders in the rust dir", err), test_map)
                    } {
                        let entry = match entry{
                            Ok(entry) => entry,
                            Err(err) => return (BridgeResult::err("failed to unwrap rust entry", err), test_map)
                        };
                        let name = match entry.file_name().into_string(){
                            Ok(entry) => entry,
                            Err(err) => return (BridgeResult::err("failed to convert osstring to String", err), test_map)
                        };
                        if name.contains("src") {
                            folders_to_search.push(entry.path());
                            key_map.insert(entry.path(), format!("{}/rust/{}", selected, name));
                        }
                    }
                } else {
                    folders_to_search.push(entry.path());
                    key_map.insert(entry.path(), format!("{}/{}", selected, name));
                }
            }
        }
    }

    for folder in folders_to_search {
        for entry in match fs::read_dir(folder.clone()){
            Ok(entry) => entry,
            Err(err) => return (BridgeResult::err("failed to get a list of files in the src or test dirs", err), test_map)
        } {
            let mut current_test = None;
            let mut last_start = 0;
            let mut tests_counted: Vec<String> = Vec::new();
            let mut not_implemented: Vec<String> = Vec::new();
            let mut comment_block = false;

            let entry = match entry{
                Ok(entry) => entry,
                Err(err) => return (BridgeResult::err("failed to unwrap src entry", err), test_map)
            };
            let name = match entry.file_name().into_string(){
                Ok(entry) => entry,
                Err(err) => return (BridgeResult::err("failed to convert ossting to String", err), test_map)
            };
            if name.contains(".dart"){
                type_of_file = ".dart".to_string();
            } else if name.contains(".rs") {
                type_of_file = ".rs".to_string();
            } else {
                continue
            }
            test_map.insert(format!("{}/{}", key_map[&folder], name), vec![]);


            let file = match File::open(entry.path()){
                Ok(file) => file,
                Err(err) => return (BridgeResult::err("failed to read source file", err), test_map)
            };
            let buffered_lines = BufReader::new(file).lines();
            let lines: Vec<String> = buffered_lines.map(|line| match line {
                Ok(line) => line,
                Err(err) => "there was an error unwrapping the source line".to_string()
            }).collect();

            if lines.contains(&"there was an error unwrapping the source line".to_string()) {
                return (BridgeResult::err("there was a error inside a closure", "error unwrapping line of source in count tests"), test_map)
            }
            let mut line_index = 0;
            'lines: loop {
                if lines[line_index].contains("/*") {
                    comment_block = true;
                }
                if lines[line_index].contains("*/")
                && comment_block == true {
                    comment_block = false;
                }
                if lines[line_index].trim().starts_with("//") {
                    line_index += 1;
                    continue 'lines;
                }
                if lines[line_index].contains("_test")
                    && current_test.is_none()
                    && comment_block == false
                    && !lines[line_index].contains(".txt")
                    && !lines[line_index].contains(".expect(")
                    //should add here if you want to exclude more false positives
                    && !lines[line_index].contains("flutter")
                    && !lines[line_index].contains("serial")
                {
                    let words = lines[line_index].split(" ");
                    for word in words {
                        if word.contains("_test") {
                            if !tests_counted.contains(&word.to_string()) 
                            && !not_implemented.contains(&word.to_string()){
                                current_test = Some(word.to_string());
                                last_start = line_index + 1;
                            }
                        }
                    }
                } else if current_test.is_some()
                    && lines[line_index].contains(&current_test.clone().unwrap()) 
                    && type_of_file == ".rs" 
                    && comment_block == false
                    && lines[line_index].contains("().expect(") 
                {
                    test_count += 1;
                    tests_counted.push(current_test.clone().unwrap());
                    let stripped_name = clean_name(&type_of_file, &current_test.unwrap());
                    match test_map.get_mut(&format!("{}/{}", key_map[&folder], name)){
                        Some(value) => value.push(stripped_name),
                        None => return (BridgeResult::err("no key or vec in the test_map at:", format!("{}/{}", key_map[&folder], name)), test_map)
                    }
                    current_test = None;
                    line_index = last_start;
                    continue 'lines;
                } else if current_test.is_some()
                    && lines[line_index].contains("expect(")
                    && type_of_file == ".dart" 
                    && comment_block == false
                {   
                    test_count += 1;
                    tests_counted.push(current_test.clone().unwrap());
                    let stripped_name = clean_name(&type_of_file, &current_test.clone().unwrap());
                    match test_map.get_mut(&format!("{}/{}", key_map[&folder], name)){
                        Some(value) => value.push(stripped_name),
                        None => return (BridgeResult::err("no key or vec in the test_map at:", format!("{}/{}", key_map[&folder], name)), test_map)
                    }
                    current_test = None;
                    line_index = last_start;

                    continue 'lines;    

                }
                if lines.len() - 1 == line_index 
                && current_test.is_none() {
                    break 'lines;
                } if lines.len() - 1 == line_index
                && current_test.is_some() {
                    not_implemented.push(current_test.clone().unwrap());
                    line_index = last_start;
                    current_test = None;
                } else {
                    line_index += 1;
                }
            }
            line_index = 0;
        }
    }
    return (BridgeResult::ok(test_count), test_map)
}

fn clean_name(filetype: &String, line: &String) -> String {
    let mut stripped_name = String::new();
    if filetype == ".rs" {
        let trimmed_name = line.trim();
        let char_to_strip = vec!["(", ")"];
        //the 42 string doesnt get dropped here which is why we have 42 string
        for character in trimmed_name.chars() {
            if !char_to_strip.contains(&&format!("{}", character)[..]) {
                stripped_name.push(character);
            }
        }
    } else {
        let trimmed_name = line.trim();
        let stripped_list = trimmed_name.split("\'");
        //the 42 string gets dropped here which is why we have an empty string
        for part in stripped_list {
            if part.contains("_test") {
                stripped_name = part.to_string();
            }
        }

    }
    return stripped_name
}

pub fn switch(tx: Sender<(String)>, rx: Receiver<(String, String)>) {
    let mut shutdown = false;
    while !shutdown {
        //failed to receive on binary_switch
        let (function, arguments) = rx.recv().expect("failed to receive function and arguments from ui thread");
        let result = match function.as_str() {
            "binary_switch" => binary_switch(&arguments),
            "emotify" => emotify(&arguments),
            "pause" => pause(&arguments),
            "play" => play(&arguments),
            "store_input" => store_input(&arguments),
            "change_project" => change_project(&arguments),
            "start" => start(&arguments),
            "exit" => {shutdown = true; exit(&arguments)},
            "idle_time" => idle_time(&arguments),
            _ => BridgeResult::new("cannot find rust function branch matching {}", function)
        };

        let output = match serde_json::to_string(&result) {
            Ok(output) => output,
            Err(_) => "{'result' : 'Err()', 'data': 'failed exit encoding!!!'}".to_string()
        };
        tx.send(output).expect("failed to send the output back to the ui thread");
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    
    #[test]
    fn test_count_folder_test() {
        let mut project_map = HashMap::new();
        project_map.insert("test_count_folder".to_string(), "/Users/j/Desktop/Code/clock/test_count_folder".to_string());
        let projects = Projects {
            map: project_map
        };

        let selected = "test_count_folder".to_string();

        let (output, test_map) = count_tests(&projects, &selected);

        if output.result != "Ok()" {
            panic!(format!("the result of the count_test function was not Ok() it was: {}", output.data[0]))
        }
        let count = &output.data[0];

        /* projected contents of test_map
        test_count_folder/src/main.rs, test_counter_test
        test_count_folder/rust/src/main.rs, test_counter_test
        test_count_folder/lib/main.dart, test_counter_test
        test_count_folder/test/widget_test.dart, test_counter_test
        test_count_folder/test_driver/app_test.dart, test_counter_test
        */
        /* projected contents of count
        5
        */


        if count != "5" {
            panic!(format!("count map did contain the folder but the count was off, it was {}, map was: \n{:?}", count, test_map));
        }

        if !test_map.contains_key("test_count_folder/src/main.rs") {
            panic!(format!("test_map did not contain the test_count_folder/src/main.rs key: \n {:?}", test_map));
        } else {
            if !test_map["test_count_folder/src/main.rs"].contains(&"test_counter_test".to_string()) {
                panic!(format!("test_map did not contain the test_counter_test value at the test_count_folder/src/main.rs key: \n {:?}", test_map));
            }
        }

        if !test_map.contains_key("test_count_folder/rust/src/main.rs") {
            panic!(format!("test_map did not contain the test_count_folder/rust/src/main.rs key: \n {:?}", test_map));
        } else {
            if !test_map["test_count_folder/rust/src/main.rs"].contains(&"test_counter_test".to_string()) {
                panic!(format!("test_map did not contain the test_counter_test value at the test_count_folder/rust/src/main.rs key: \n{:?}", test_map));
            }
        }

        if !test_map.contains_key("test_count_folder/lib/main.dart") {
            panic!(format!("test_map did not contain the test_count_folder/lib/main.dart key: \n {:?}", test_map));
        } else {
            if !test_map["test_count_folder/lib/main.dart"].contains(&"test_counter_test".to_string()) {
                panic!(format!("test_map did not contain the test_counter_test value at the test_count_folder/lib/main.dart key: \n {:?}", test_map));
            }
        }

        if !test_map.contains_key("test_count_folder/test/widget_test.dart") {
            panic!(format!("test_map did not contain the test_count_folder/test/widget_test.dart key: \n {:?}", test_map));
        } else {
            if !test_map["test_count_folder/test/widget_test.dart"].contains(&"test_counter_test".to_string()) {
                panic!(format!("test_map did not contains the test_counter_test value at the test_count_folder/test/widget_test.dart key: \n {:?}", test_map));
            }
        }

        if !test_map.contains_key("test_count_folder/test_driver/app_test.dart") {
            panic!(format!("test_map did not contain the test_count_folder/test_driver/app_test.dart key: \n {:?}", test_map));
        } else {
            if !test_map["test_count_folder/test_driver/app_test.dart"].contains(&"test_counter_test".to_string()) {
                panic!(format!("test_map did not contain the test_counter_test value at the test_count_folder/test_driver/app_test.dart: \n {:?}", test_map));
            }
        }
    }

    #[test]
    fn test_count_external_test_with_comments_0() {
        //this is where we go through and count the current tests in other folders
        //and expect it and good output as well

        //then we take any errors here and build them into the test_count_folder

        //rusty_flutter has 
        // lib.rs 7 with 1 not implemented (no expect later)
        // main.dart 0
        // widget_test.dart 1 with 4 expects
        // app_test.dart 10 with 2 not implemented (_test with no expect)
        // tests including the comments

        //missed 4 from lib.rs
        //caught the two non implemented from app_test.dart
        //didnt get any from widget_test.dart (should be 1)


        //need to have one key be all or if you want complication not_counted
        //not counted requires that we know the position of each following expect (counts a test too early before no expect is found)

        let mut project_map = HashMap::new();
        project_map.insert("rusty_flutter".to_string(), "/Users/j/Desktop/Code/rusty_flutter".to_string());
        let projects = Projects {
            map: project_map
        };

        let selected = "rusty_flutter".to_string();

        let (output, test_map) = count_tests(&projects, &selected);

        if output.result != "Ok()" {
            panic!(format!("the result of the count_test function was not Ok() it was: {}", output.data[0]))
        }
        let count = &output.data[0];

        let lib_rs = "rusty_flutter/rust/src/lib.rs".to_string();
        if !test_map.contains_key(&lib_rs) {
            panic!(format!("test_map did not contain the {} key: \n {:?}", lib_rs, test_map));
        } else {
            if !test_map[&lib_rs].contains(&"store_one_test".to_string()) {
                panic!(format!("test_map did not contain the store_one_test value at the {} key: \n {:?}", lib_rs, test_map));
            }
            if !test_map[&lib_rs].contains(&"store_many_test".to_string()) {
                panic!(format!("test_map did not contain the store_many_test value at the {} key: \n {:?}", lib_rs, test_map));
            }
            if !test_map[&lib_rs].contains(&"store_many_different_test".to_string()) {
                panic!(format!("test_map did not contain the store_many_different_test value at the {} key: \n {:?}", lib_rs, test_map));
            }
            if !test_map[&lib_rs].contains(&"search_one_test".to_string()) {
                panic!(format!("test_map did not contain the search_one_test value at the {} key: \n {:?}", lib_rs, test_map));
            }
        
            if !test_map[&lib_rs].contains(&"search_many_different_test".to_string()) {
                panic!(format!("test_map did not contain the search_many_different_test value at the {} key: \n {:?}", lib_rs, test_map));
            }
            if test_map[&lib_rs].contains(&"search_many".to_string()) {
                panic!(format!("test_map did contain a non implemented search_many test at the {} key: \n {:?}", lib_rs, test_map));
            }
            if !test_map[&lib_rs].contains(&"create_table_with_one_column_test".to_string()) {
                panic!(format!("test_map did not contain the create_tamle_with_one_column_test value at the {} key: \n {:?}", lib_rs, test_map));
            }


        }
        if count != "13" {
            panic!(format!("count map did contain the folder but the count was off, it was {}, map was: \n{:?}", count, test_map));
        }
        //STEP NOTES
        //it seems to have an empty string for the lib folder but it is there it just starts with a \x06 (maybe '/') and the 'U' char is a /0
        //instead of just a rust restarter on pub mod tests, just keep the line number of the last test (fn not expect) and reset the line only back to there + 1
        //it turns out it was looking for expects for lines that contained expect (like searching for each test twice)
        //so we caught search_one_test just by making sure .txt and .expect lines were never included to be searched for
    }

    #[test]
    fn test_count_external_test_with_comments_1() {
        let mut project_map = HashMap::new();
        project_map.insert("FinSCRAPE".to_string(), "/Users/j/Desktop/Code/FinSCRAPE".to_string());
        let projects = Projects {
            map: project_map
        };

        let selected = "FinSCRAPE".to_string();

        let (output, test_map) = count_tests(&projects, &selected);

        if output.result != "Ok()" {
            panic!(format!("the result of the count_test function was not Ok() it was: {}", output.data[0]))
        }
        let count = &output.data[0];

        if count != "20" {
            panic!(format!("count map did contain the folder but the count was off, it was {}, map was: \n{:?}", count, test_map));
        }

    }

}

//rules for db
// must sign in from projects.txt
// or ask for credentials
// should pull down on first start for the day
// should push up each time the db is closed
// should also have a sync key
// should error gracefully into results
// might make it so we copy to usb in the event that internet is not working
// and usb should error gracefully with "please insert a usb drive to sync locally"