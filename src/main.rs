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

const TASK_FORCES: &'static str = "task_forces";
const STRIKE_FORCES: &'static str = "strike_forces";
const COOP: &'static str = "coop";
const TRIALS: &'static str = "trials";
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
                name,
                level_requirement,
                merits,
                team_size,
                location,
                tips,
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
#[serde(tag = "type")]
pub enum Tip {
    General { name: String, content: String },
    Speed { name: String, content: String },
    Badge { name: String, content: String },
}

fn main() {
    let team_path = Path::new(".")
        .join(PROPERTIES_DIR)
        .join(TEAM_EVENT_FILE_NAME);
    let team_events: Vec<GroupEvent> = read_data_file(team_path);

    let league_path = Path::new(".")
        .join(PROPERTIES_DIR)
        .join("league_events.json");
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
    print!("{}", group_one_menus);
    let group_two_menus = generate_menus(&tera, g2, TEAM_EVENT_TEMPLATE);
    print!("{}", group_two_menus);

    let league_event_menus = generate_menus(&tera, league_events.clone(), LEAGUE_EVENT_TEMPLATE);

    let mut top_level_context = Context::new();
    top_level_context.insert("group_one", &group_one_menus);
    top_level_context.insert("group_two", &group_two_menus);
    top_level_context.insert("league_events", &league_event_menus);

    let result = tera.render("lfgmacros.vm", &top_level_context);
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

        let mut tip_menus = String::new();
        if !tips.is_empty() {
            let mut general_tips: Vec<Tip> = Vec::new();
            let mut speed_tips: Vec<Tip> = Vec::new();
            let mut badge_tips: Vec<Tip> = Vec::new();
            for tip in tips {
                match tip {
                    Tip::General {
                        name: _,
                        content: _,
                    } => general_tips.push(tip.clone()),
                    Tip::Speed {
                        name: _,
                        content: _,
                    } => speed_tips.push(tip.clone()),
                    Tip::Badge {
                        name: _,
                        content: _,
                    } => badge_tips.push(tip.clone()),
                }
            }

            if !general_tips.is_empty() {
                tip_menus.push_str(&generate_tips(tera, "General", &general_tips));
            }
            if !speed_tips.is_empty() {
                tip_menus.push_str(&generate_tips(tera, "Speed", &speed_tips));
            }
            if !badge_tips.is_empty() {
                tip_menus.push_str(&generate_tips(tera, "Badge", &badge_tips));
            }
        }
        println!("{}", tip_menus);

        let mut context = Context::new();
        context.insert("content", &event);
        context.insert("tips", &tip_menus);
        let result = tera.render(template_name, &context);
        match result {
            Ok(sub_menu) => menus.push_str(&sub_menu),
            Err(e) => panic!("Could not render lfg template: {:?}", e),
        };
    }

    menus
}

fn generate_tips(tera: &Tera, tip_type: &str, tips: &Vec<Tip>) -> String {
    let mut context = Context::new();
    context.insert("type", &tip_type);
    context.insert("tips", &tips);
    let result = tera.render("tips.vm", &context);
    match result {
        Ok(tip_menu) => return tip_menu,
        Err(e) => panic!("Unable to render tips {:?}", e),
    };
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
