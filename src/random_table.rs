use rltk::RandomNumberGenerator;


/// Represents counts and names of items or monsters
pub struct RandomEntry {
    name : String,
    weight : i32,
}


impl RandomEntry {
    pub fn new<S:ToString>(name : S, weight : i32) -> RandomEntry {
        RandomEntry { 
            name : name.to_string(),
            weight
        }
    }
}


#[derive(Default)]
/// Store entities and their total weight
pub struct RandomTable {
    entries : Vec<RandomEntry>,
    total_weight : i32,
}


impl RandomTable {

    /// Create new random table
    pub fn new() -> RandomTable {
        RandomTable {
            entries : Vec::new(),
            total_weight : 0,
        }
    }

    /// Add new entity to random table
    pub fn add<S:ToString>(mut self, name : S, weight : i32) -> RandomTable {
        if weight > 0 {
            self.total_weight += weight;
            self.entries.push(RandomEntry::new(name.to_string(), weight));
        }
        self
    }

    /// Randomly determines the number of different entries on the map
    pub fn roll(&self, rng : &mut RandomNumberGenerator) -> String {
        if self.total_weight == 0 {
            return "None".to_string();
        }

        let mut roll = rng.roll_dice(1, self.total_weight) - 1;
        let mut index : usize = 0;

        while roll > 0 {
            if roll < self.entries[index].weight {
                return self.entries[index].name.clone();
            }
            roll -= self.entries[index].weight;
            index += 1;
        }

        "None".to_string()
    }
}


/// Create a random table and add to it new entries
pub fn room_table(map_depth : i32) -> RandomTable {
    RandomTable::new()
        .add("Goblin", 10)
        .add("Orc", 1 + map_depth)
        .add("Health Potion", 2)
        .add("Fireball Scroll", 2 + map_depth)
        .add("Confusion Scroll", 2 + map_depth)
        .add("Magic Missible Scroll", 4)
        .add("Dagger", 3)
        .add("Shield", 3)
        .add("Longsword", map_depth - 1)
        .add("Tower Shield", map_depth - 1)
}
