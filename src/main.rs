///with the logic/ui split
/// this will be where we call both ui and switch
/// and send them both txs and rxs to and from eachother
/// we will keep all of the structs used in both apps in here
/// to reduce duplication and allow us to edit in one place
//#![feature(trace_macros)]
//trace_macros!(true);
#[allow(dead_code)]
mod logic;
use logic::switch;

mod ui;
use ui::tree;

use std::str;
use std::env;
use std::fs;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::result::Result;
use std::time::Duration;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::io;
use std::io::{Read, Write};
use std::thread;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::default::Default;


use termion::event::Key;
use termion::cursor::Goto;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::input::MouseTerminal;
use termion::screen::AlternateScreen;
use tui::Terminal;
use tui::style::{Color, Style};
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::backend::TermionBackend;
use unicode_width::UnicodeWidthStr;

use google_drive3::Drive;
use yup_oauth2::{Authenticator, DefaultAuthenticatorDelegate, ApplicationSecret, MemoryStorage};
use hyper;
use hyper_rustls;

extern crate rusqlite;
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, NO_PARAMS, MappedRows, Row, params, ToSql};

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use chrono::prelude::*;

extern crate indexmap;
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy)]
struct Config {
    exit_key: Key,
    tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

enum Event<I> {
    Input(I),
    Tick,
}

struct Events {
    rx: mpsc::Receiver<Event<Key>>,
    input_handle: thread::JoinHandle<()>,
    tick_handle: thread::JoinHandle<()>,
}

impl Events {
    fn new() -> Events {
        Events::with_config(Config::default())
    }

    fn with_config(config: Config) -> Events {
        let (tx, rx) = mpsc::channel();
        let input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    match evt {
                        Ok(key) => {
                            if let Err(_) = tx.send(Event::Input(key)) {
                                return;
                            }
                            if key == config.exit_key {
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        };
        let tick_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let tx = tx.clone();
                loop {
                    tx.send(Event::Tick).unwrap();
                    thread::sleep(config.tick_rate);
                }
            })
        };
        Events {
            rx,
            input_handle,
            tick_handle,
        }
    }

    fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
        self.rx.recv()
    }
}
#[derive(Serialize, Deserialize, Clone)]
pub struct App {
    input: String,
    comment: String,
    messages: Vec<String>,
    metrics: Vec<String>,
    selected: String,
    state: String,
    idle: f64,
    avg_idle: f64,
    tests_per_hour: f64,
    result: Vec<String>,
    flags: String,
    mood: i64,
    water: i64,
    caffeine: i64,
    thc: i64,
    nicotine: i64,
    food: i64,
    alcohol: i64,
    hungover: i64,
    learning: i64,
    tests: i64,
    test_map: IndexMap<String, i64>,
    current_project: String,
    projects: Projects,
    idle_vec: Vec<String>,
    test_vec: Vec<f64>,
    time_vec: Vec<String>,
    meds: i64
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            comment: String::new(),
            messages: Vec::new(),
            metrics: Vec::new(),
            state: "play".to_string(),
            selected: String::new(),
            avg_idle: 0.0,
            idle: 0.0,
            tests_per_hour: 0.0,
            result: vec![],
            flags: "😁 • 💧0 ☕0 🚬0 🍁0 🍞0 🥃0 • ➕".to_string(),
            mood: 1,
            water: 0,
            caffeine: 0,
            nicotine: 0,
            thc: 0,
            food: 0,
            alcohol: 0,
            hungover: 0,
            meds: 0,
            learning: 1,
            tests: 0,
            test_map: IndexMap::new(),
            current_project: String::new(),
            projects: Projects::default(),
            idle_vec: Vec::new(),
            test_vec: Vec::new(),
            time_vec: Vec::new()
        }
        //might be able to init db here
    }
}

#[derive(Serialize)]
struct ToInit {
    table: String,
    columns: String,
    path: String
}

#[derive(Serialize)]
struct ToStore {
    table: String,
    data: Vec<String>,
    path: String

}

#[derive(Debug)]
struct FromDB {
    mood: String,
    learning: String,
    water: String,
    caffeine: String,
    nicotine: String,
    thc: String,
    food: String,
    alcohol: String,
    hungover: String,
    meds: String,
    time: i64
}

struct FromDBs {
    tests: String
}

#[derive(Serialize, Deserialize, Clone)]
struct Projects {
    map: HashMap<String, String>
}

impl Default for Projects {
    fn default() -> Projects {
        Projects {
            map: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct BridgeResult {
    result: String,
    data: Vec<String>
}

struct BridgeResultMaps {
    result: String,
    data: Vec<HashMap<String, String>>
}

impl Default for BridgeResult {
    fn default() -> BridgeResult {
        BridgeResult {
            result: "".to_string(),
            data: vec!["".to_string()]

        }
    }
}

impl BridgeResult {
    fn new(result: &'static str, data: String) -> BridgeResult {
        BridgeResult {
            result: result.to_string(),
            data: vec![data.to_string()]
        }
    }

    fn err<E: std::fmt::Debug>(desc: &'static str, err: E) -> BridgeResult {
        //this should write a log of every error
        let mut file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open("log.txt").expect("failed to open log");

        let local: DateTime<Local> = Local::now();

        file.write(format!("{} ///{}: {:?}\n", local.date(), desc, err).as_bytes()).expect("failed to write log");

        BridgeResult {
            result: "Err()".to_string(),
            data: vec![format!("{}: {:?}", desc, err)]
        }
    }
    //std::string::ToString
    fn ok<D: std::string::ToString>(data: D) -> BridgeResult {
        BridgeResult {
            result: "Ok()".to_string(),
            data: vec![data.to_string()]
        }
    }

    fn ok_strings(data: Vec<String>) -> BridgeResult {
        BridgeResult {
            result: "Ok()".to_string(),
            data: data
        }
    }

    fn ok_maps(data: Vec<HashMap<String, String>>) -> BridgeResultMaps {
        BridgeResultMaps {
            result: "Ok()".to_string(),
            data: data
        }
    }
}

trait TypeInfo {
    fn type_of(&self) -> String;
}

impl TypeInfo for Vec<String> {
    fn type_of(&self) -> String {
        "Vec<String>".to_string()
    }
}

impl TypeInfo for String {
    fn type_of(&self) -> String {
        "String".to_string()
    }
}

impl TypeInfo for &'static str {
    fn type_of(&self) -> String {
        "&str".to_string()
    }
}


fn main() {
    let (to_logic, from_ui) = mpsc::channel();
    let (to_ui, from_logic) = mpsc::channel();

    let mut app = initialize();

    let logic_thread = thread::spawn(move || {
        switch(to_ui, from_ui); 
    });

    //ui continues on main thread
    tree(to_logic, from_logic, &mut app);
    logic_thread.join().expect("failed to join logic thread after shutdown");

}

fn initialize() -> App {
    //init app struct and pass only it back, with 
    let mut app = App::default();
    let mut last_data = FromDB {
        mood: "0".to_string(),
        learning: "0".to_string(),
        water: "0".to_string(),
        caffeine: "0".to_string(),
        nicotine: "0".to_string(),
        thc: "0".to_string(),
        food: "0".to_string(),
        alcohol: "0".to_string(),
        hungover: "0".to_string(),
        meds: "0".to_string(),
        time: 0
    };

    let storage = Connection::open(
        format!("{}/clock.db", env::current_dir().expect("failed to get current dir in initialize").display().to_string())
    ).expect("failed to open a connection to storage in initialize");


    loop {
        println!("what is the project you will be working on ?>");
        let mut selected = String::new();
        io::stdin().read_line(&mut selected).expect("failed to get project name from stdin");
        selected = selected.trim().to_string();

        let projects = get_config().expect("failed to get config, check layout");

        if !projects.map.contains_key(&selected) {
            println!("the selected project was not found, try adding it the json file !>");
            continue
        }

        
        let mut last_tests = FromDBs {
            tests: "0".to_string(),
        };

        let mut test_map = IndexMap::new();

        for selection in projects.map.keys() {
            //if clock.db is already there do read tests instead
            let to_init = ToInit {
                table: selection.clone(),
                columns: "project, state, comment, previous_commit, tests, mood, learning, doses, water, caffeine, nicotine, thc, food, alcohol, hungover, meds, hour, minute, second, day, month, year, unixtime".to_string(),
                path: env::current_dir().expect("failed to get current directory in initialize").display().to_string()
            };

            let init_string = serde_json::to_string(&to_init).expect("failed to decode init string");

            let output = init_storage(&init_string);
            if output.result == "Err()" {
                panic!(format!("{:?}", output.data));
            }
        }

        for selection in projects.map.keys() {
            let query = format!("SELECT * FROM total WHERE unixtime = (SELECT MAX(unixtime) FROM {})", selection);

            let mut statement = storage.prepare(&query).expect("failed to prepare max time test query in initialize");

            let rows = statement.query_map(NO_PARAMS, |r| Ok(
                FromDBs {
                    tests: match r.get(4) {
                        Ok(column) => column,
                        //this one wont let me return bridgeresult
                        Err(err) => format!("error getting column: {}", err)
                    }
                }
            )).expect("failed to get rows from test time query map in initialize");
            let mut count = 0;
            for row in rows {
                last_tests = row.expect("failed to unwrap test time row in initialize");
                count += 1;
            }
            if count != 0 {
                test_map.insert(selection.to_owned(), last_tests.tests.parse::<i64>().expect("failed to parse test count from string in initialize"));
            } else {
                test_map.insert(selection.to_owned(), 0);
            }

        }


        let local: DateTime<Local> = Local::now();
        let query = format!("SELECT * from total WHERE month='{}' AND day='{}' AND year='{}'", local.month(), local.day(), local.year());

        let mut statement = storage.prepare(&query).expect("failed to prepare day resume query in initialize");

        let any_rows = statement.exists(NO_PARAMS).expect("failed to query database for day resume");


        if !any_rows {
            //this is where we will trigger a download
            //then we will run the above date check again and come back through these
            //with an if on a bool called updated to skip the download after it has been done once
            println!("Did you take your pill today ?>");
            let mut inBOX = String::new();
            io::stdin().read_line(&mut inBOX).expect("failed to get meds input from std in in initialize");

            if inBOX.to_lowercase().starts_with("y") {
                last_data.meds = "1".to_string();
            }

            println!("Have you eaten today ?>");
            let mut inBOX = String::new();
            io::stdin().read_line(&mut inBOX).expect("failed to get food input from std in in initialize");
            if inBOX.to_lowercase().starts_with("y") {
                last_data.food = "1".to_string();
            }

            println!("Did you drink last night ?>");
            let mut inBOX = String::new();
            io::stdin().read_line(&mut inBOX).expect("failed to get hangover input from std in in initialize");

        } else {
            let query = format!("SELECT * FROM total WHERE unixtime = (SELECT MAX(unixtime) FROM total)");
            let mut statement = storage.prepare(&query).expect("failed to prepare max time resume query in initialize");
            //cannot infer lifetime
            let rows = statement.query_map(NO_PARAMS, |r| Ok(
                FromDB {
                    mood: r.get(5).expect("failed to get mood column"), 
                    learning: r.get(6).expect("failed to get learning column"), 
                    water: r.get(8).expect("failed to get water column"), 
                    caffeine: r.get(9).expect("failed to get caffeine column"), 
                    nicotine: r.get(10).expect("failed to get nicotine column"), 
                    thc: r.get(11).expect("failed to get thc column"), 
                    food: r.get(12).expect("failed to get food column"), 
                    alcohol: r.get(12).expect("failed to get alcohol column"), 
                    hungover: r.get(13).expect("failed to get hungover column"), 
                    meds: r.get(14).expect("failed to get meds column"), 
                    time: r.get(22).expect("failed to get time column")
                }
            )).expect("failed to query db");

            for row in rows {
                last_data = row.expect("failed to unwrap row");
            }
            //panic!("last_data: {:?}", last_data);                        
        }
        app.selected = selected;
        app.test_map = test_map;
        app.projects = projects;

        app.food = last_data.food.parse::<i64>().expect("failed to parse food string from db");
        app.meds = last_data.meds.parse::<i64>().expect("failed to parse meds string from db");
        app.hungover = last_data.hungover.parse::<i64>().expect("failed to parse hangover string from db");

        if last_data.time > 0 {
            app.mood = last_data.mood.parse::<i64>().expect("failed to parse mood string from db");
            app.learning = last_data.learning.parse::<i64>().expect("failed to parse learning string from db");
            app.water = last_data.water.parse::<i64>().expect("failed to parse water string from db");
            app.caffeine = last_data.caffeine.parse::<i64>().expect("failed to parse caffeine string from db");
            app.nicotine = last_data.nicotine.parse::<i64>().expect("failed to parse nicotine string from db");
            app.thc = last_data.thc.parse::<i64>().expect("failed to parse thc string from db");
        }

        break
    }
    storage.close().expect("failed to close db connection");
    app
}

fn get_config() -> Result<Projects, ()> {
    let input = fs::read_to_string("projects.txt").expect("failed to read json input file");

    let projects: Projects = serde_json::from_str(&input).expect("failed to decode json file");
    
    Ok(projects)
}

fn init_storage(data: &String) -> BridgeResult {
    #[derive(Deserialize)]
    struct Arguments {
        table: String,
        columns: String,
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

    let table_statement = format!("CREATE TABLE IF NOT EXISTS {} ({})", arguments.table, arguments.columns);

    match storage.execute(&table_statement, NO_PARAMS) {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to create table: {:?}", err)
    };

    match storage.close() {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to close the db: {:?}", err)
    };

    let storage = match Connection::open(format!("{}/clock.db", arguments.path)) {
        Ok(storage) => storage,
        Err(err) => return BridgeResult::err("failed to open connection to db: {:?}", err)
    };

    let table_statement = format!("CREATE TABLE IF NOT EXISTS total ({})", arguments.columns);

    match storage.execute(&table_statement, NO_PARAMS) {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to create table: {:?}", err)
    };

    match storage.close() {
        Ok(_) => (),
        Err(err) => return BridgeResult::err("failed to close the db: {:?}", err)
    };

    BridgeResult::ok(
        format!("created the table: {} successfully with columns: {}", arguments.table, arguments.columns)
    )
}
