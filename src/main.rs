use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path;
use std::path::Path;
use std::path::PathBuf;
use std::vec::Vec;
use tera::Context;
use tera::Tera;

const PROPERTIES_DIR: &str = "properties";
const TEMPLATES: &str = "templates";
const TEAM_EVENT_FILE_NAME: &str = "team_events.json";
const LEAGUE_EVENT_FILE_NAME: &str = "league_events.json";

const TOP_LEVEL_MENU_TEMPLATE: &str = "lfgmacros.vm";
const TEAM_EVENT_TEMPLATE: &str = "task_strike_trial_lfg.vm";
const LEAGUE_EVENT_TEMPLATE: &str = "league_lfg.vm";

const OUTPUT_FILE_NAME: &'static str = "lfgmacros.mnu";

const LEVEL_SPIT: &'static u8 = &36;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GroupEvent {
    TeamEvent {
        name: String,
        level_requirement: u8,
        merits: u8,
        team_size: u8,
        location: String,
        tips: Vec<Tip>,
    },
    LeagueEvent {
        name: String,
        requirements: String,
        rewards: String,
        league_size: u8,
        location: String,
        tips: Vec<Tip>,
    },
}
impl GroupEvent {
    fn compare_level_requirement(&self, value: &u8) -> i8 {
        match self {
            GroupEvent::TeamEvent {
                name: _,
                level_requirement,
                merits: _,
                team_size: _,
                location: _,
                tips: _,
            } => {
                if level_requirement < value {
                    -1
                } else if value == &0 {
                    0
                } else {
                    1
                }
            }
            _ => panic!("Not a team event"),
        }
    }
    fn get_tips(&self) -> &Vec<Tip> {
        match self {
            GroupEvent::TeamEvent {
                name: _,
                level_requirement: _,
                merits: _,
                team_size: _,
                location: _,
                tips,
            } => tips,
            GroupEvent::LeagueEvent {
                name: _,
                requirements: _,
                rewards: _,
                league_size: _,
                location: _,
                tips,
            } => tips,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tip {
    name: String,
    content: String,
}

// Load the data from the JSON files
// For each team/league event are generated in this order
// tip macros->tip menus->LFG menus->event menu->top level menu
// It's done this way because of the way the menus have to be nested
fn main() {
    let team_events = load_json_data(PROPERTIES_DIR, TEAM_EVENT_FILE_NAME);
    let league_events = load_json_data(PROPERTIES_DIR, LEAGUE_EVENT_FILE_NAME);

    // Because there are so many team events, sort them into two groups based on minimal level
    let (g1, g2): (_, Vec<GroupEvent>) = team_events
        .into_iter()
        .partition(|e| e.compare_level_requirement(LEVEL_SPIT) < 0);

    let tera = match Tera::new(&format!("{}{}*.vm", TEMPLATES, path::MAIN_SEPARATOR)) {
        Ok(t) => t,
        Err(e) => panic!("Unable to load templates: {:?}", e),
    };

    let group_one_menus = generate_menus(&tera, g1, TEAM_EVENT_TEMPLATE);
    //print!("{}", group_one_menus);
    let group_two_menus = generate_menus(&tera, g2, TEAM_EVENT_TEMPLATE);
    //print!("{}", group_two_menus);
    let league_event_menus = generate_menus(&tera, league_events.clone(), LEAGUE_EVENT_TEMPLATE);

    // Generate the top level menu given all the sub menus
    let mut top_level_context = Context::new();
    top_level_context.insert("group_one", &group_one_menus);
    top_level_context.insert("group_two", &group_two_menus);
    top_level_context.insert("league_events", &league_event_menus);

    let result = tera.render(TOP_LEVEL_MENU_TEMPLATE, &top_level_context);
    match result {
        Ok(top_menu) => write_output(top_menu),
        Err(e) => panic!("Could not render top level template: {:?}", e),
    }
}

fn load_json_data(properties_dir: &str, file_name: &str) -> Vec<GroupEvent> {
    let file_path = Path::new(".").join(properties_dir).join(file_name);
    read_data_file(file_path)
}

fn generate_menus(tera: &Tera, group_events: Vec<GroupEvent>, template_name: &str) -> String {
    let mut menus: String = String::new();
    for event in group_events {
        let tips = event.get_tips();

        let mut tip_macros = String::new();
        if !tips.is_empty() {
            for tip in tips {
                // Macros in menus are even more restricted than what is available in the chat window
                // The macro names cannot have spaces
                // There cannot be any double or single quotes inside the macro
                // Whitespace is allowed after the macro name
                // "macro name content$$macro name content"
                // Ex: Option "Generate Tip Macros" "macro CadaverCounterBadge say Cadaver Counter Badge: Defeat the Vahzilok leader in the Death from Below Sewer Trial without killing any of the Cadavers. That means no AOE, Sands of Mu is an AOE, park/dismiss your pets, all damage on Dr. Meinst till she is down.$$macro TheCleanserBadge say The Cleanser Badge: Defeat all the Lost Worshippers in the Death from Below Sewer Trial before defeating the Lost leader."
                // If you don't follow the format the macros will fail silently
                tip_macros.push_str(&format!(
                    "macro {} say {}$$",
                    tip.name
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>(),
                    tip.content
                ));
            }
        }
        let mut context = Context::new();
        context.insert("content", &event);
        if !tips.is_empty() {
            // truncate the last $$
            tip_macros.truncate(tip_macros.chars().count() - 2);
            context.insert("tips", &tips);
            context.insert("tip_macros", &tip_macros);
        }
        let result = tera.render(template_name, &context);
        match result {
            Ok(sub_menu) => menus.push_str(&sub_menu),
            Err(e) => panic!("Could not render lfg template: {:?}", e),
        };
    }

    menus
}

fn read_data_file(path: PathBuf) -> Vec<GroupEvent> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) => panic!("Error {:?}", e),
    };
    let reader = BufReader::new(file);

    let data: Vec<GroupEvent> = match serde_json::from_reader(reader) {
        Ok(data) => data,
        Err(e) => panic!("Error {:?}", e),
    };

    data
}

fn write_output(contents: String) {
    fs::write(OUTPUT_FILE_NAME, contents).expect("Unable to write file");
}
