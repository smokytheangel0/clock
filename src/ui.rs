//this is where we will put the ui only
use super::*;

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

#[derive(Serialize)]
struct ToLogicApp {
    app: App
}

#[derive(Serialize)]
struct ToLogicValue {
    value: i64
}

pub fn tree(tx: Sender<(String, String)>, rx: Receiver<(String)>, mut app: &mut App) {
    let stdout = io::stdout().into_raw_mode().expect("failed to put stdout into raw mode");
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("failed to setup termion");

    let events = Events::new();

    //emotify call
    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

    let mut mood = String::new();
    let mut learning = String::new();

    if emotify_output.result != "Ok()" {
        push_error(emotify_output.data, &mut app);
    } else {
        mood = emotify_output.data[0].clone();
        learning = emotify_output.data[1].clone();
    }
    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
    //end emotify

    //start call
    let output_app = AtoA_BridgeCall("start", &mut app, &tx, &rx);
    app.tests = output_app.tests;
    push_error(output_app.result, &mut app);
    //end start

    loop {
        //idle_time call
        let output_app = AtoA_BridgeCall("idle_time", &mut app, &tx, &rx);
        app.idle = output_app.idle;
        push_error(output_app.result, &mut app);
        //end idle_time


        if app.idle > 180.0 && app.state == "play" {
            //pause call
            let output_app = AtoA_BridgeCall("pause", &mut app, &tx, &rx);
            app.tests = output_app.tests;
            app.state = output_app.state;
            push_error(output_app.result, &mut app);
            //end pause
        }
        if app.state == "pause" && app.idle < 180.0 {
            //play call
            let app_struct = ToLogicApp {
                app: app.clone()
            };
            let app_string = match serde_json::to_string(&app_struct) {
                Ok(string) => string,
                Err(err) => {app.result.push(format!("Err(): failed to encode app string for play call: {}", err)); String::new()}
            };

            match tx.send(("play".to_string(), app_string)) {
                Ok(_) => (),
                Err(err) => {app.result.push(format!{"Err(): failed to send play call and data to logic thread: {}", err})}
            };

            let output_string = match rx.recv() {
                Ok(output) => output,
                Err(err) => {app.result.push(format!("Err(): failed to receive result from play call: {}", err)); String::new()}
            };

            let play_output: BridgeResult = match serde_json::from_str(&output_string) {
                Ok(output) => output,
                Err(err) => {app.result.push(format!("Err(): failed to decode result string from play call: {}", err)); BridgeResult::default()}
            };

            if play_output.result != "Ok()" {
                app.result.push(play_output.data[0].clone());
            } else {
                let mut output_app: App = match serde_json::from_str(&play_output.data[0]) {
                    Ok(output) => output,
                    Err(err) => {app.result.push(format!("Err(): failed to decode app struct from play call: {}", err)); App::default()}
                };
                app.tests = output_app.tests;
                app.state = output_app.state;
                for result in output_app.result.clone() {
                    if !app.result.contains(&result) {
                        app.result.append(& mut output_app.result);
                    }
                }
            }
            //end play
        }

        if app.tests == 0 {
            app.tests = app.test_map[&app.selected];
        }

        //the actual tree
        terminal.draw(|mut f| {

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                //length(3) perc(80) perc(20)
                .constraints([Constraint::Length(3), Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
                .split(f.size());
            Paragraph::new([Text::raw(&app.input)].iter())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Comment"))
                .render(&mut f, chunks[0]);            
            {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                    .split(chunks[1]);

                let messages = app
                    .messages
                    .iter()
                    .enumerate()
                    .map(|(i, m)| Text::raw(format!("{}: {}", i, m)));
                List::new(messages)
                    .block(Block::default().borders(Borders::ALL).title("Task History"))
                    .render(&mut f, chunks[0]);
                
                app.metrics = vec![
                    format!("project: {}", app.selected),
                    format!("idle state: {}", app.state),
                    format!("idle: {}", app.idle),
                    format!("test count: {}", app.tests),
                ];

                let metrics = app
                    .metrics
                    .iter()
                    .map(|m| Text::raw(m));

                List::new(metrics)
                    .block(Block::default().borders(Borders::ALL).title("Metrics"))
                    .render(&mut f, chunks[1]);                
            }
            {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
                    .split(chunks[2]);

                let results = app
                    .result
                    .iter()
                    .map(|m| Text::raw(m));

                List::new(results)
                    .block(Block::default().borders(Borders::ALL).title("Results"))
                    .render(&mut f, chunks[0]);                
            
                Paragraph::new([Text::raw(&app.flags)].iter())
                    .style(Style::default().fg(Color::Yellow))
                    .block(Block::default().borders(Borders::ALL).title("Flags"))
                    .render(&mut f, chunks[1]); 
                             
            }
        }).expect("failed to do layout");

        write!(
            terminal.backend_mut(),
            "{}",
            Goto(3 + app.input.width() as u16, 3)
        ).expect("failed to reposition cursor to input box");

        match events.next().expect("failed to get next event") {
            Event::Input(input) => match input {

                Key::Ctrl('c') => {
                    //exit call
                    let output_app = AtoA_BridgeCall("exit", &mut app, &tx, &rx);
                    app.tests = output_app.tests;
                    app.state = output_app.state;
                    //end exit
                    break;
                }

                Key::Char('\n') => {
                    app.comment = app.input.drain(..).collect();
                    let local: DateTime<Local> = Local::now();
                    if app.comment == "" {
                        app.comment = "updated flags".to_string();
                    }
                    app.messages.push(format!("{} {} '{}' T: {}:{}:{}", app.selected.clone(), app.state.clone(), app.comment.clone(), local.hour(), local.minute(), local.second()));
                    
                    //store_input call
                    let output_app = AtoA_BridgeCall("store_input", &mut app, &tx, &rx);
                    app.tests = output_app.tests;
                    push_error(output_app.result, &mut app);
                    //end store_input
                }

                Key::Char(c) => {
                    app.input.push(c);
                }

                Key::Backspace => {
                    app.input.pop();
                }

                //- and i are blocked from use probably a terminal mapping
                Key::Ctrl('a') => {
                    //binary_switch call
                    let binary_output = VtoV_BridgeCall("binary_switch", app.mood.clone(), &mut app, &tx, &rx);

                    if binary_output.result != "Ok()" {
                        app.result.push(binary_output.data[0].clone());
                    } else {
                        app.mood = binary_output.data[0].clone().parse::<i64>().expect("failed to parse mood int from logic result");
                    }
                    //end binary_switch

                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('d') => {
                    app.water += 1;
                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('f') => {
                    app.alcohol += 1;
                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                }
                Key::Ctrl('h') => {
                    app.caffeine += 1;
                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('t') => {
                    app.nicotine += 1;
                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('n') => {
                    app.thc += 1;
                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('s') => {
                    app.food += 1;
                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        push_error(emotify_output.data, &mut app);
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('o') => {
                    //binary_switch call
                    let binary_output = VtoV_BridgeCall("binary_switch", app.learning.clone(), &mut app, &tx, &rx);

                    if binary_output.result != "Ok()" {
                        app.result.push(binary_output.data[0].clone());
                    } else {
                        app.learning = binary_output.data[0].clone().parse::<i64>().expect("failed to parse learning int from logic result");
                    }
                    //end binary_switch

                    //emotify call
                    let emotify_output = AtoV_BridgeCall("emotify", &mut app, &tx, &rx);

                    let mut mood = String::new();
                    let mut learning = String::new();

                    if emotify_output.result != "Ok()" {
                        app.result.push(emotify_output.data[0].clone());
                    } else {
                        mood = emotify_output.data[0].clone();
                        learning = emotify_output.data[1].clone();
                    }
                    app.flags = format!("{} • 💧{} ☕{} 🚬{} 🍁{} 🍞{} 🥃{} • {}", mood, app.water, app.caffeine, app.nicotine, app.thc, app.food, app.alcohol, learning);
                    //end emotify
                }
                Key::Ctrl('p') => {
                    //change_project call
                    let output_app = AtoA_BridgeCall("change_project", &mut app, &tx, &rx);
                    app.tests = output_app.tests;
                    app.state = output_app.state;
                    app.selected = output_app.selected;
                    push_error(output_app.result, &mut app);
                    //end change_project


                    if app.tests == 0 {
                        app.tests = app.test_map[&app.selected];
                    }
                    
                }
                _ => {}
            },
            _ => {}
        }
    }
    
}

//BridgeCall can be app to app, app to value and value to value
//make sure to change references to state to value so the naming stays straight
//push_error(output_app.result)


//AtoA_BridgeCall
//AtoV_BridgeCall
//VtoV_BridgeCall

fn AtoA_BridgeCall(function: &'static str, mut app: &mut App, tx: &Sender<(String, String)>, rx: &Receiver<(String)> ) -> App {
    let mut output_app = App::default();
    let app_struct = ToLogicApp {
        app: app.clone()
    };
    let app_string = match serde_json::to_string(&app_struct) {
        Ok(string) => string,
        Err(err) => {push_error(vec![format!("Err(): failed to encode app string for {}: {}", function, err)], &mut app); String::new()}
    };

    match tx.send((function.to_string(), app_string)) {
        Ok(_) => (),
        Err(err) => {push_error(vec![format!{"Err(): failed to send {} call and data to logic thread: {}", function, err}], &mut app)}
    };

    let output_string = match rx.recv() {
        Ok(output) => output,
        Err(err) => {push_error(vec![format!("Err(): failed to receive result from {} call: {}", function, err)], &mut app); String::new()}
    };

    let bridge_output: BridgeResult = match serde_json::from_str(&output_string) {
        Ok(output) => output,
        Err(err) => {push_error(vec![format!("Err(): failed to decode result string from {} call: {}", function, err)], &mut app); BridgeResult::default()}
    };

    if bridge_output.result != "Ok()" {
        push_error(bridge_output.data, &mut app);
    } else {
        output_app = match serde_json::from_str(&bridge_output.data[0]) {
            Ok(output) => output,
            Err(err) => {push_error(vec![format!("Err(): failed to decode app struct from {} call: {}", function, err)], &mut app); App::default()}
        };
    }
    output_app
}

fn AtoV_BridgeCall(function: &'static str, mut app: &mut App, tx: &Sender<(String, String)>, rx: &Receiver<(String)>) -> BridgeResult {
    let app_struct = ToLogicApp {
        app: app.clone()
    };
    let app_string = match serde_json::to_string(&app_struct) {
        Ok(string) => string,
        Err(err) => {push_error(vec![format!("Err(): failed to encode app string for {}: {}", function, err)], &mut app); String::new()}
    };

    match tx.send((function.to_string(), app_string)) {
        Ok(_) => (),
        Err(err) => {push_error(vec![format!{"Err(): failed to send {} call and data to logic thread: {}", function, err}], &mut app)}
    };

    let output_string = match rx.recv() {
        Ok(output) => output,
        Err(err) => {push_error(vec![format!("Err(): failed to receive result from {} call: {}", function, err)], &mut app); String::new()}
    };

    let bridge_output: BridgeResult = match serde_json::from_str(&output_string) {
        Ok(output) => output,
        Err(err) => {push_error(vec![format!("Err(): failed to decode result string from {} call: {}", function, err)], &mut app); BridgeResult::default()}
    };
    bridge_output
}

fn VtoV_BridgeCall(function: &'static str, value: i64, mut app: &mut App, tx: &Sender<(String, String)>, rx: &Receiver<(String)>) -> BridgeResult {
    let value_struct = ToLogicValue {
        value: value
    };
    let state_string = match serde_json::to_string(&value_struct) {
        Ok(string) => string,
        Err(err) => {push_error(vec![format!("Err(): failed to encode value string for {}: {}", function, err)], &mut app); String::new()}
    };

    match tx.send((function.to_string(), state_string)) {
        Ok(_) => (),
        Err(err) => {push_error(vec![format!("Err(): failed to send {} call and data to logic thread: {}", function, err)], &mut app)}
    };

    let output_string = match rx.recv() {
        Ok(output) => output,
        Err(err) => {push_error(vec![format!("Err(): failed to receive result from {} call: {}", function, err)], &mut app); String::new()}
    };

    let value_output: BridgeResult = match serde_json::from_str(&output_string) {
        Ok(output) => output,
        Err(err) => {push_error(vec![format!("Err(): failed to decode result string from {} call: {}", function, err)], &mut app); BridgeResult::default()}
    };
    value_output
}

fn push_error(results: Vec<String>, app: &mut App) {
    for result in results {
        if !app.result.contains(&result) {
            app.result.push(result);
        }
    }
}

