//! Lane D4 (the desktop): the subsystems theme (dungeons and the initiative, Backgrounds, emblems,
//! dice, the Ring, sagas, split cards). Conventions: docs/lane-brief.md. Card text:
//! assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d4.rs; the core pieces' own tests are in
//! tests/core_subsystems.rs.
#![allow(unused_imports)]

use super::*;
use bolt_core::SubsystemEffect;

/// A land, with this controller restriction.
const fn land(controller: Controller) -> Filter {
    Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        lands: true,
        controller,
        ..Filter::ANY
    }
}

/// "target permanent"
const PERMANENT: Filter = Filter {
    players: false,
    any_permanent: true,
    ..Filter::ANY
};

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Split cards (CR 709): the right half is cast through the Omen path ----
        "Boom // Bust" => s(
            &[
                "Destroy target land you control and target land you don't control.",
                "Split: Destroy all lands.",
            ],
            |d, _| {
                spell(
                    d,
                    vec![land(Controller::You), land(Controller::Opponent)],
                    vec![Effect::Destroy(0), Effect::Destroy(1)],
                );
                omen(d, Ability::new(vec![], vec![Effect::DestroyAllLands]));
            },
        ),
        "Fire // Ice" => s(
            &[
                "Fire deals 2 damage divided as you choose among one or two targets.",
                "Split: Tap target permanent.",
                "Split: Draw a card.",
            ],
            |d, _| {
                let mut fire = up_to(vec![Filter::ANY, Filter::ANY], vec![Effect::DividedDamage]);
                fire.min_targets = Some(1);
                fire.divide = 2;
                d.spell = Some(fire);
                omen(d, Ability::new(vec![PERMANENT], vec![Effect::Tap(0), draw(1)]));
            },
        ),
        // ---- Dungeons (game/d4.rs) ----
        "Hama Pashar, Ruin Seeker" => s(
            &["Room abilities of dungeons you own trigger an additional time."],
            |d, _| d.subsystems.rooms_trigger_again = true,
        ),
        "Dungeon Delver" => s(
            &["Commander creatures you own have \"Room abilities of dungeons you own trigger an additional time.\""],
            |d, _| d.subsystems.commanders_rooms_trigger_again = true,
        ),
        // ---- Dice ----
        "Ancient Gold Dragon" => s(
            &["Whenever this creature deals combat damage to a player, roll a d20. You create a number of 1/1 blue Faerie Dragon creature tokens with flying equal to the result."],
            |d, t| {
                let roll = SubsystemEffect::RollDieCreateTokens { sides: 20, def: t.faerie_dragon };
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::Subsystem(roll)]);
            },
        ),
        _ => None,
    }
}

pub const PG_D4_SCRIPTED: &[&str] = &[
    // Split cards
    "Boom // Bust",
    "Fire // Ice",
    // Dungeons
    "Hama Pashar, Ruin Seeker",
    "Dungeon Delver",
    // Dice
    "Ancient Gold Dragon",
];
