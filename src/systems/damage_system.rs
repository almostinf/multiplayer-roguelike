use specs::prelude::*;
use crate::{CombatStats, SufferDamage, ClientHandler, Player, Name, GameLog, RunState};


/// Reduction of xp after any hit
pub struct DamageSystem<'a> {
    pub enemies : Vec<String>,
    pub game_client : &'a mut ClientHandler,
}


impl<'a> System<'a> for DamageSystem<'a> {
    type SystemData = ( 
                        WriteStorage<'a, CombatStats>,
                        WriteStorage<'a, SufferDamage>,
                        ReadStorage<'a, Name>,
                    );

    fn run(&mut self, data : Self::SystemData) {
        let (mut stats, mut damage, names) = data;

        for (mut stats, damage, name) in (&mut stats, &damage, &names).join() {
            stats.hp -= damage.amount.iter().sum::<i32>();

            // Sending a message to the server to notify the other players of the hp change
            if self.enemies.iter().find(|&_name| *_name == name.name) != None {
                let message = format!("{{\"__DAMAGE__\":\"{} {}\"}}", name.name, stats.hp).as_bytes().to_vec();
                self.game_client.send_message(message);
            }
        }

        damage.clear();
    }
}

/// Delete all dead entities
pub fn delete_the_dead(ecs : &mut World) {
    let mut dead : Vec<Entity> = Vec::new();

    // Using a scope to make borrow checker happy
    {
        let combat_stats = ecs.read_storage::<CombatStats>();
        let entities = ecs.entities();
        let names = ecs.read_storage::<Name>();
        let players = ecs.read_storage::<Player>();
        let mut log = ecs.write_resource::<GameLog>();
        for (entity, stats) in (&entities, &combat_stats).join() {
            if stats.hp < 1 {
                let player = players.get(entity);
                
                // game over if player is a dead entity
                match player {
                    None => {
                        let victim_name = names.get(entity);
                        if let Some(victim_name) = victim_name {
                            log.entries.push(format!("{} is dead", &victim_name.name));
                        }
                        dead.push(entity);
                    }
                    Some(_) => {
                        let mut runstate = ecs.write_resource::<RunState>();
                        *runstate = RunState::GameOver;
                    }
                }
            }
        }
    }

    for victim in dead {
        ecs.delete_entity(victim).expect("Unable to delete");
    }
}
