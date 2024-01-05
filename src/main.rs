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

const PROPERTIES_DIR: &'static str = "properties";
const TEMPLATES: &'static str = "templates";
const TEAM_EVENT_FILE_NAME: &'static str = "team_events.json";
const LEAGUE_EVENT_FILE_NAME: &'static str = "league_events.json";

const TOP_LEVEL_MENU_TEMPLATE: &'static str = "lfgmacros.vm";
const TEAM_EVENT_TEMPLATE: &'static str = "task_strike_trial_lfg.vm";
const LEAGUE_EVENT_TEMPLATE: &'static str = "league_lfg.vm";

const OUTPUT_FILE_NAME: &'static str = "lfgmacros.mnu";

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
    fn compare_level_requirement(self, value: u8) -> i8 {
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
                } else if value == 0 {
                    0
                } else {
                    1
                }
            }
            _ => panic!("Not a team event"),
        }
    }
    fn get_tips(&self) -> &Vec<Tip> {
        let result = match self {
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
        };

        result
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Tip {
    name: String,
    content: String,
}

fn main() {
    let team_path = Path::new(".")
        .join(PROPERTIES_DIR)
        .join(TEAM_EVENT_FILE_NAME);
    let team_events: Vec<GroupEvent> = read_data_file(team_path);

    let league_path = Path::new(".")
        .join(PROPERTIES_DIR)
        .join(LEAGUE_EVENT_FILE_NAME);
    let league_events: Vec<GroupEvent> = read_data_file(league_path);

    let (g1, g2): (_, Vec<GroupEvent>) = team_events
        .clone()
        .into_iter()
        .partition(|e| e.clone().compare_level_requirement(36) < 0);

    let tera = match Tera::new(&format!("{}{}*.vm", TEMPLATES, path::MAIN_SEPARATOR)) {
        Ok(t) => t,
        Err(e) => panic!("Unable to load templates: {:?}", e),
    };

    let group_one_menus = generate_menus(&tera, g1, TEAM_EVENT_TEMPLATE);
    //print!("{}", group_one_menus);
    let group_two_menus = generate_menus(&tera, g2, TEAM_EVENT_TEMPLATE);
    //print!("{}", group_two_menus);

    let league_event_menus = generate_menus(&tera, league_events.clone(), LEAGUE_EVENT_TEMPLATE);

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

fn generate_menus(
    tera: &Tera,
    group_events: Vec<GroupEvent>,
    template_name: &'static str,
) -> String {
    let mut menus: String = String::new();
    for event in group_events {
        let tips = event.get_tips();

        let mut tip_macros = String::new();
        if !tips.is_empty() {
            for tip in tips {
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
