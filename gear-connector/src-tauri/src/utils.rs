use std::{
    fmt::{self},
    sync::atomic::AtomicBool,
};

use gear_connector_api::SecondarySkill;
use tauri::Window;
use tracing::{
    field::{Field, Visit},
    span, Event, Metadata, Subscriber,
};
use tracing_core::Level;
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

pub struct MainWindowSubscriber {
    pub window: Window,
}
struct MessageFormatter {
    message: String,
}

impl fmt::Display for MessageFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct StringVisitor<'a> {
    string: &'a mut String,
}

impl<'a> Visit for StringVisitor<'a> {
    fn record_debug(&mut self, _field: &Field, value: &dyn fmt::Debug) {
        use std::fmt::Write;
        write!(self.string, "{:?}", value).unwrap();
    }
    fn record_str(&mut self, _field: &Field, value: &str) {
        use std::fmt::Write;
        write!(self.string, "{}", value).unwrap();
    }
}

impl<S> Layer<S> for MainWindowSubscriber
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut string = String::new();
        let mut visitor = StringVisitor {
            string: &mut string,
        };
        event.record(&mut visitor);
        match event.metadata().level() {
            &Level::DEBUG => {
                // self.main_window.emit("debug", string).unwrap();
            }
            &Level::INFO => {
                self.window.emit("log", string).unwrap();
            }
            &Level::WARN => {
                self.window.emit("warn", string).unwrap();
            }
            &Level::ERROR => {
                self.window.emit("error", string).unwrap();
            }
            _ => {}
        }
    }
}

impl Subscriber for MainWindowSubscriber {
    fn new_span(&self, _span: &span::Attributes) -> span::Id {
        println!("NEW SPAN");
        // self.main_window.emit("log", )

        println!("VALUE = {}", _span.values().to_string());

        span::Id::from_u64(1)
    }

    fn record(&self, _: &span::Id, _: &span::Record) {}

    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn enter(&self, _: &span::Id) {}

    fn exit(&self, _: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        println!("EVENT: {:?}", event);
        // let s: String =     format!()
    }
}

pub fn convert_state(
    player_state: gear_connector_api::PlayerState,
) -> homm3_gamestate_io::PlayerState {
    let heroes: Vec<homm3_gamestate_io::Hero> = player_state
        .heroes
        .into_iter()
        .map(|hero| convert_hero(hero))
        .collect();

    let resources: Vec<homm3_gamestate_io::Resource> = player_state
        .resources
        .into_iter()
        .map(|resource| convert_resource(resource))
        .collect();

    let towns: Vec<homm3_gamestate_io::Town> = player_state
        .towns
        .into_iter()
        .map(|town| convert_town(town))
        .collect();

    homm3_gamestate_io::PlayerState {
        color: player_state.color,
        team_id: player_state.team_id,
        is_human: player_state.is_human,
        resources,
        heroes,
        towns,
        days_without_castle: player_state.days_without_castle,
    }
}

pub fn convert_secondary_skill_info(
    info: gear_connector_api::SecondarySkillInfo,
) -> homm3_gamestate_io::SecondarySkillInfo {
    homm3_gamestate_io::SecondarySkillInfo {
        skill: convert_secondary_skill(info.skill),
        value: info.value,
    }
}

pub fn convert_secondary_skill(
    secondary_skill: SecondarySkill,
) -> homm3_gamestate_io::SecondarySkill {
    match secondary_skill {
        SecondarySkill::Wrong => homm3_gamestate_io::SecondarySkill::Wrong,
        SecondarySkill::Default => homm3_gamestate_io::SecondarySkill::Default,
        SecondarySkill::Pathfinding => homm3_gamestate_io::SecondarySkill::Pathfinding,
        SecondarySkill::Archery => homm3_gamestate_io::SecondarySkill::Archery,
        SecondarySkill::Logistics => homm3_gamestate_io::SecondarySkill::Logistics,
        SecondarySkill::Scouting => homm3_gamestate_io::SecondarySkill::Scouting,
        SecondarySkill::Diplomacy => homm3_gamestate_io::SecondarySkill::Diplomacy,
        SecondarySkill::Navigation => homm3_gamestate_io::SecondarySkill::Navigation,
        SecondarySkill::Leadership => homm3_gamestate_io::SecondarySkill::Leadership,
        SecondarySkill::Wisdom => homm3_gamestate_io::SecondarySkill::Wisdom,
        SecondarySkill::Mysticism => homm3_gamestate_io::SecondarySkill::Mysticism,
        SecondarySkill::Luck => homm3_gamestate_io::SecondarySkill::Luck,
        SecondarySkill::Ballistics => homm3_gamestate_io::SecondarySkill::Ballistics,
        SecondarySkill::EagleEye => homm3_gamestate_io::SecondarySkill::EagleEye,
        SecondarySkill::Necromancy => homm3_gamestate_io::SecondarySkill::Necromancy,
        SecondarySkill::Estates => homm3_gamestate_io::SecondarySkill::Estates,
        SecondarySkill::FireMagic => homm3_gamestate_io::SecondarySkill::FireMagic,
        SecondarySkill::AirMagic => homm3_gamestate_io::SecondarySkill::AirMagic,
        SecondarySkill::WaterMagic => homm3_gamestate_io::SecondarySkill::WaterMagic,
        SecondarySkill::EarthMagic => homm3_gamestate_io::SecondarySkill::EarthMagic,
        SecondarySkill::Scholar => homm3_gamestate_io::SecondarySkill::Scholar,
        SecondarySkill::Tactics => homm3_gamestate_io::SecondarySkill::Tactics,
        SecondarySkill::Artillery => homm3_gamestate_io::SecondarySkill::Artillery,
        SecondarySkill::Learning => homm3_gamestate_io::SecondarySkill::Learning,
        SecondarySkill::Offence => homm3_gamestate_io::SecondarySkill::Offence,
        SecondarySkill::Armorer => homm3_gamestate_io::SecondarySkill::Armorer,
        SecondarySkill::Intelligence => homm3_gamestate_io::SecondarySkill::Intelligence,
        SecondarySkill::Sorcery => homm3_gamestate_io::SecondarySkill::Sorcery,
        SecondarySkill::Resistance => homm3_gamestate_io::SecondarySkill::Resistance,
        SecondarySkill::FirstAid => homm3_gamestate_io::SecondarySkill::FirstAid,
        SecondarySkill::SkillSize => homm3_gamestate_io::SecondarySkill::SkillSize,
    }
}

pub fn convert_hero(hero: gear_connector_api::Hero) -> homm3_gamestate_io::Hero {
    let secondary_skills: Vec<homm3_gamestate_io::SecondarySkillInfo> = hero
        .secondary_skills
        .into_iter()
        .map(|info| convert_secondary_skill_info(info))
        .collect();
    let mut stacks: [Option<homm3_gamestate_io::Stack>; 7] = Default::default();
    for (i, stack) in hero.stacks.into_iter().enumerate() {
        stacks[i] = convert_stack(stack);
    }
    homm3_gamestate_io::Hero {
        name: hero.name,
        level: hero.level,
        mana: hero.mana,
        sex: hero.sex,
        experience_points: hero.experience_points,
        secondary_skills,
        stacks,
    }
}

pub fn convert_resource(resource: gear_connector_api::Resource) -> homm3_gamestate_io::Resource {
    match resource {
        gear_connector_api::Resource::Wood(v) => homm3_gamestate_io::Resource::Wood(v),
        gear_connector_api::Resource::Mercury(v) => homm3_gamestate_io::Resource::Mercury(v),
        gear_connector_api::Resource::Ore(v) => homm3_gamestate_io::Resource::Ore(v),
        gear_connector_api::Resource::Sulfur(v) => homm3_gamestate_io::Resource::Sulfur(v),
        gear_connector_api::Resource::Crystal(v) => homm3_gamestate_io::Resource::Crystal(v),
        gear_connector_api::Resource::Gems(v) => homm3_gamestate_io::Resource::Gems(v),
        gear_connector_api::Resource::Gold(v) => homm3_gamestate_io::Resource::Gold(v),
        gear_connector_api::Resource::Mithril(v) => homm3_gamestate_io::Resource::Mithril(v),
        gear_connector_api::Resource::WoodAndOre => homm3_gamestate_io::Resource::WoodAndOre,
        gear_connector_api::Resource::Invalid => homm3_gamestate_io::Resource::Invalid,
    }
}

fn convert_town(town: gear_connector_api::Town) -> homm3_gamestate_io::Town {
    homm3_gamestate_io::Town {
        name: town.name,
        fort_level: convert_fort_level(town.fort_level),
        hall_level: convert_hall_level(town.hall_level),
        mage_guild_level: town.mage_guild_level,
        level: town.level,
    }
}

fn convert_fort_level(level: gear_connector_api::FortLevel) -> homm3_gamestate_io::FortLevel {
    match level {
        gear_connector_api::FortLevel::None => homm3_gamestate_io::FortLevel::None,
        gear_connector_api::FortLevel::Fort => homm3_gamestate_io::FortLevel::Fort,
        gear_connector_api::FortLevel::Citadel => homm3_gamestate_io::FortLevel::Citadel,
        gear_connector_api::FortLevel::Castle => homm3_gamestate_io::FortLevel::Castle,
    }
}

fn convert_hall_level(level: gear_connector_api::HallLevel) -> homm3_gamestate_io::HallLevel {
    match level {
        gear_connector_api::HallLevel::None => homm3_gamestate_io::HallLevel::None,
        gear_connector_api::HallLevel::Village => homm3_gamestate_io::HallLevel::Village,
        gear_connector_api::HallLevel::Town => homm3_gamestate_io::HallLevel::Town,
        gear_connector_api::HallLevel::City => homm3_gamestate_io::HallLevel::City,
        gear_connector_api::HallLevel::Capitol => homm3_gamestate_io::HallLevel::Capitol,
    }
}

fn convert_stack(stack: Option<gear_connector_api::Stack>) -> Option<homm3_gamestate_io::Stack> {
    if let Some(stack) = stack {
        Some(homm3_gamestate_io::Stack {
            name: stack.name,
            level: stack.level,
            count: stack.count,
        })
    } else {
        None
    }
}
