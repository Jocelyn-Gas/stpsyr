extern crate csv;

use std::fmt;
use std::cmp;

// the only information attached to a Unit is its owner and type
// ex. "Austrian fleet"
#[derive(Clone)]
pub struct Unit {
    pub owner: Power,
    pub unit_type: UnitType
}
impl fmt::Debug for Unit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} {:?}", self.unit_type, self.owner)
    }
}
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum UnitType { Army, Fleet }

// a Province is an extension of a String, partially for semantics, but also
//   because we need to take coasts into account when enumerating borders
#[derive(Clone)]
pub struct Province {
    name: String,
    coast: Option<char>,
    from_coast: Option<char>
}
impl fmt::Debug for Province {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.name,
            self.coast.map_or(String::new(), |coast| format!("/{}c", coast)),
            self.from_coast.map_or(String::new(), |coast| format!(" [from {}c]", coast)))
    }
}
impl From<String> for Province {
    fn from(s: String) -> Province {
        if let Some(idx) = s.find('/') {
            let mut s = s;
            let coast = s.chars().nth(idx + 1);
            s.truncate(idx);
            Province { name: s, coast: coast, from_coast: None }
        } else {
            Province { name: s, coast: None, from_coast: None }
        }
    }
}
impl<'a> From<&'a str> for Province {
    fn from(s: &str) -> Province {
        Province::from(s.to_string())
    }
}
impl cmp::PartialEq for Province {
    fn eq(&self, other: &Province) -> bool {
        self.name == other.name
    }
}

// a Power is simply a wrapper around a String for semantics
// ex. Germany, Austria
#[derive(Clone)]
pub struct Power {
    name: String
}
impl fmt::Debug for Power {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
impl From<String> for Power {
    fn from(s: String) -> Power {
        Power { name: s }
    }
}
impl<'a> From<&'a str> for Power {
    fn from(s: &str) -> Power {
        Power::from(s.to_string())
    }
}
impl cmp::PartialEq for Power {
    fn eq(&self, other: &Power) -> bool {
        self.name.to_lowercase() == other.name.to_lowercase()
    }
}

// a MapRegion is a location on the map, storing the province, whether it's an
//   SC, its current owner, the unit in it (not necessarily with the same owner
//   as the region), and its borders (stored separately for fleets and armies)
#[derive(Clone)]
struct MapRegion {
    province: Province,
    sc: bool,
    owner: Option<Power>,
    unit: Option<Unit>,
    fleet_borders: Vec<Province>,
    army_borders: Vec<Province>
}
impl fmt::Debug for MapRegion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}{}{}{}",
            self.province,
            if self.sc { "*" } else { "" },
            self.owner.as_ref().map_or(String::new(), |o| format!(" ({:?})", o)),
            self.unit.as_ref().map_or(String::new(), |o| format!(" [{:?}]", o)))
    }
}
impl cmp::PartialEq for MapRegion {
    fn eq(&self, other: &MapRegion) -> bool {
        self.province == other.province
    }
}

// here are some utility types for the Order struct
#[derive(Clone,Debug,PartialEq)]
enum OrderState { UNRESOLVED, GUESSING, RESOLVED }
#[derive(Clone,Debug)]
pub enum Action {
    Hold,
    Move { to: Province, convoyed: bool },
    SupportHold { to: Province },
    SupportMove { from: Province, to: Province },
    Convoy { from: Province, to: Province }
}

// an Order stores the power that ordered it, which province is being ordered,
//   the actual order (action), and some meta information for the resolve() and
//   adjudicate() functions
// it is separate from a Retreat and an Adjust
#[derive(Clone,Debug)]
struct Order {
    owner: Power,
    province: Province,
    action: Action,
    resolution: bool,
    state: OrderState,
    id: usize
}

// utility type for Retreat, corresponding to Action for Order
pub enum RetreatAction {
    Disband,
    Move { to: Province }
}

// a Retreat stores the power that ordered it, which province to retreat from,
//   and what to do with it (disband or move)
struct Retreat {
    owner: Power,
    province: Province,
    action: RetreatAction
}

pub enum AdjustAction {
    Disband,
    Build { unit_type: UnitType }
}

// a Adjust stores the power that ordered it, which province to build/destroy
// in, and what to do there (disband or build a unit)
struct Adjust {
    owner: Power,
    province: Province,
    action: AdjustAction
}

// fairly self-explanatory
#[derive(Clone,Copy,Debug,PartialEq)]
enum Phase {
    SpringDiplomacy,
    SpringRetreats,
    FallDiplomacy,
    FallRetreats,
    Builds
}

// this is the main struct (duh)
pub struct Stpsyr {
    map: Vec<MapRegion>,
    orders: Vec<Order>,
    retreats: Vec<Retreat>,
    adjusts: Vec<Adjust>,
    dependencies: Vec<usize>,
    dislodged: Vec<(Province, Unit)>,
    phase: Phase,
    year: i32
}

impl Stpsyr {
    pub fn new(mapfile: &'static str) -> Stpsyr {
        // parse input file as CSV to generate the map
        let mut reader = csv::Reader::from_file(mapfile).unwrap();

        let mut map: Vec<MapRegion> = Vec::new();
        for region in reader.decode::<(
                    String,          // 0 name
                    bool,            // 1 SC?
                    Option<String>,  // 2 starting owner
                    Option<String>,  // 3 starting unit type
                    String,          // 4 bordering provinces (fleets)
                    String           // 5 bordering provinces (armies)
                )>() {
            let region = region.unwrap();
            let province = Province::from(region.0.clone());

            let fleet_borders: Vec<Province> = region.4.split_whitespace().map(|p| {
                let mut border = Province::from(p);
                if let Some(coast) = province.coast {
                    border.from_coast = Some(coast);
                }
                border
            }).collect();
            let army_borders = region.5.split_whitespace().map(Province::from)
                .collect();

            if let Some(existing_region) = map.iter_mut()
                    .find(|r| r.province == province) {
                existing_region.fleet_borders.extend(fleet_borders.iter().cloned());
                continue;
            }

            map.push(MapRegion {
                province: province,
                sc: region.1,

                owner: region.2.clone().map(Power::from),
                unit: region.3.as_ref().map(|unit_type| Unit {
                    owner: Power::from(region.2.clone().unwrap()),
                    unit_type: match &unit_type[..] {
                        "Army" => UnitType::Army,
                        "Fleet" => UnitType::Fleet,
                        _ => panic!("unit type must be Army or Fleet")
                    }
                }),

                fleet_borders: fleet_borders,
                army_borders: army_borders
            });
        };

        Stpsyr {
            map: map,
            orders: vec![],
            retreats: vec![],
            adjusts: vec![],
            dependencies: vec![],
            dislodged: vec![],
            phase: Phase::SpringDiplomacy,
            year: 1901
        }
    }

    // the publicly exposed function to modify self.orders
    pub fn add_order(&mut self, owner: Power, province: Province, action: Action) {
        // there has to be a unit here to order it
        let unit = if let Some(unit) = self.get_unit(&province) { unit }
            else { return; };

        let (is_move, convoyed) = match action {
            Action::Move { ref to, convoyed } => {
                // let's do a quick check here: unit can't move to itself
                if province == *to { return; }
                (true, convoyed)
            },
            Action::SupportMove { ref from, ref to } => {
                // another quick check: can't support yourself or a non-move
                if province == *from || province == *to || *from == *to { return; }
                (false, false)
            }
            _ => (false, false)
        }; // TODO use this better

        // can't convoy a fleet
        if convoyed && unit.unit_type == UnitType::Fleet { return; }

        // can't order a unit that's not yours
        if unit.owner != owner { return; }

        // can't order to a province you can't reach
        if !convoyed && match &action {
            &Action::Move { ref to, convoyed: _ } |
            &Action::SupportHold { ref to } |
            &Action::SupportMove { from: _, ref to } => {
                let r = self.get_region(&province).unwrap();
                !match unit.unit_type {
                    UnitType::Army => r.army_borders.clone(),
                    UnitType::Fleet => r.fleet_borders.clone().into_iter()
                        .filter(|p|
                            p.from_coast == r.province.coast &&
                            (!is_move || p.coast == to.coast))
                        .collect()
                }.contains(&to)
            },
            _ => false
        } { return; }

        // all checks pass
        let id = self.orders.len();
        self.orders.push(Order {
            owner: owner,
            province: province,
            action: action,
            resolution: false,
            state: OrderState::UNRESOLVED,
            id: id
        });
    }

    // this is the publicly exposed function that is called once all orders
    //   have been added
    pub fn apply_orders(&mut self) -> Vec<(Province, Unit)> {
        // resolve all orders
        for i in 0..self.orders.len() {
            self.resolve(i);
            assert!(self.orders[i].state == OrderState::RESOLVED);
            println!("{:?}", self.orders[i]);
        }

        // do the moves that were successfully resolved
        self.apply_resolved();

        // update phase data
        // TODO don't go to a phase if nobody has stuff to do for it
        self.phase = match self.phase {
            Phase::SpringDiplomacy => if self.dislodged.is_empty() {
                Phase::FallDiplomacy
            } else {
                Phase::SpringRetreats
            },
            Phase::SpringRetreats => Phase::FallDiplomacy,
            Phase::FallDiplomacy | Phase::FallRetreats =>
                if self.phase == Phase::FallRetreats || self.dislodged.is_empty() {
                    Phase::Builds // TODO only if people have them
                } else {
                    Phase::FallRetreats
                },
            Phase::Builds => { self.year += 1; Phase::SpringDiplomacy }
        };

        println!("{:?} {}: {:?}", self.phase, self.year, self.map);

        // clear out orders, return dislodged units
        self.orders = vec![];
        self.dislodged.clone()
    }

    // the publicly exposed function to modify self.retreats
    pub fn add_retreat(&mut self, owner: Power, province: Province, action: RetreatAction) {
        self.retreats.push(Retreat {
            owner: owner,
            province: province,
            action: action
        });
    }

    // the publicly exposed function that is called once all retreats have been
    //   added
    pub fn apply_retreats(&mut self) {
        // TODO
    }

    // the publicly exposed function to modify self.adjusts
    pub fn add_adjust(&mut self, owner: Power, province: Province, action: AdjustAction) {
        self.adjusts.push(Adjust {
            owner: owner,
            province: province,
            action: action
        });
    }

    // the publicly exposed function that is called once all adjusts have been
    //   added
    pub fn apply_adjusts(&mut self) {
        // TODO
    }

    // this is the function that actually moves units when their resolution is
    //   successful
    fn apply_resolved(&mut self) {
        // anything that got moved on top of (but maybe it also moved away)
        let mut dislodged: Vec<(Province, Unit)> = vec![];
        // anything that left an empty space (but maybe something also moved in)
        let mut moved_away: Vec<&Province> = vec![];

        let old_map = self.map.clone();
        for order in self.orders.iter() { if order.resolution {
            match order.action { Action::Move { ref to, convoyed: _ } => {
                // we have a successful move
                let from_idx = self.map.iter()
                    .position(|r| r.province == order.province).unwrap();
                let to_idx = self.map.iter()
                    .position(|r| r.province == *to).unwrap();

                if let Some(ref unit) = self.map[to_idx].unit {
                    dislodged.push((to.clone(), unit.clone()));
                }

                self.map[to_idx].unit = old_map[from_idx].unit.clone();
                if !self.map[to_idx].sc || self.phase == Phase::FallDiplomacy {
                    self.map[to_idx].owner = old_map[from_idx].owner.clone();
                }

                if let Some(_) = to.coast {
                    self.map[to_idx].province.coast =
                        self.map[to_idx].province.coast.and(to.coast);
                }

                moved_away.push(&order.province);
            }, _ => {} }
        } }

        // now we can do processing for dislodged and moved_away
        for region in self.map.iter_mut() {
            let p_dislodged = dislodged.iter().find(|d| d.0 == region.province);
            let p_moved_away = moved_away.contains(&&region.province);
            if let Some(dislodgement) = p_dislodged {
                if !p_moved_away {
                    // dislodged and not moved away: add it to the list
                    self.dislodged.push(dislodgement.clone());
                }
            } else if p_moved_away {
                // moved away and not dislodged: clear from map
                region.unit = None;
            }
        }
    }

    // parse orders as a string and apply them
    pub fn parse_orders(&mut self, orders: String) {
        let mut power = Power::from(String::new());

        for line in orders.lines() {
            let line = line.to_lowercase()
                .replace('-', " ")
                .replace(" m ", " ")
                .replace(" move ", " ")
                .replace(" move to ", " ")
                .replace(" moves ", " ")
                .replace(" moves to ", " ")
                .replace('(', " ")
                .replace(')', " ")
                .replace(" support ", " s ")
                .replace(" supports ", " s ")
                .replace("via convoy", "vc")
                .replace(" convoy ", " c ")
                .replace(" convoys ", " c ");
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.is_empty() { continue; }
            else if tokens.len() == 1 {
                power = Power::from(tokens.into_iter().next().unwrap());
                continue;
            } else {
                let mut tokens_iter = tokens.iter().filter(|&token|
                    *token != "a" &&
                    *token != "army" &&
                    *token != "f" &&
                    *token != "fleet" &&
                    *token != "h" &&
                    *token != "hold" &&
                    *token != "holds" &&
                    *token != "stand" &&
                    *token != "stands");
                let province = Province::from(*tokens_iter.next().unwrap());
                match tokens_iter.next() {
                    None => {}, // hold
                    Some(token2) => { match *token2 {
                        "s" => {
                            // support
                            let a = tokens_iter.next().unwrap();
                            if let Some(b) = tokens_iter.next() {
                                // support move
                                self.add_order(power.clone(), province, Action::SupportMove {
                                    from: Province::from(*a), to: Province::from(*b)
                                });
                            } else {
                                // support hold
                                self.add_order(power.clone(), province, Action::SupportHold {
                                    to: Province::from(*a)
                                });
                            }
                        },
                        "c" => {
                            // convoy
                            let from = tokens_iter.next().unwrap();
                            let to = tokens_iter.next().unwrap();
                            self.add_order(power.clone(), province, Action::Convoy {
                                from: Province::from(*from), to: Province::from(*to)
                            });
                        },
                        _ => {
                            // regular move
                            let vc = tokens_iter.next().map_or(false, |token|
                                *token == "vc");
                            self.add_order(power.clone(), province, Action::Move {
                                to: Province::from(*token2), convoyed: vc
                            });
                        }
                    } }
                }
            }
        }
    }

    // get the unit currently in a province
    pub fn get_unit(&self, province: &Province) -> Option<Unit> {
        self.get_region(province).and_then(|r| r.unit.clone())
    }

    // get the MapRegion corresponding to a provence
    fn get_region(&self, province: &Province) -> Option<&MapRegion> {
        self.map.iter().find(|r| r.province == *province)
    }

    // this is the recursive resolve function, almost directly copied from
    //   http://diplom.org/Zine/S2009M/Kruijswijk/DipMath_Chp6.htm
    // it takes the id of an order and returns whether it was successful
    fn resolve(&mut self, id: usize) -> bool {
        match self.orders[id].state {
            // if order is already resolved, just return the resolution
            OrderState::RESOLVED => self.orders[id].resolution,
            OrderState::GUESSING => {
                // if we're guessing, add the order to the dependency list
                // and return the guess
                if !self.dependencies.contains(&id) {
                    self.dependencies.push(id);
                }
                self.orders[id].resolution
            },
            OrderState::UNRESOLVED => {
                let old_dep_count = self.dependencies.len();

                // start guessing
                self.orders[id].resolution = false;
                self.orders[id].state = OrderState::GUESSING;

                // adjudicate the order with the first guess
                let first_result = self.adjudicate(id);

                if self.dependencies.len() == old_dep_count {
                    // result is not dependent on a guess
                    match self.orders[id].state {
                        OrderState::RESOLVED => {},
                        _ => { self.orders[id].resolution = first_result; }
                    }
                    self.orders[id].state = OrderState::RESOLVED;
                    return first_result;
                }

                if self.dependencies[old_dep_count] != id {
                    // result is dependent on guess, but not our own
                    self.dependencies.push(id);
                    self.orders[id].resolution = first_result;
                    return first_result;
                }

                // result is dependent on our own guess, so let's guess again
                for dep in self.dependencies.drain(old_dep_count..) {
                    self.orders[dep].state = OrderState::UNRESOLVED;
                }
                self.orders[id].resolution = true;
                self.orders[id].state = OrderState::GUESSING;

                // adjudicate with the second guess
                let second_result = self.adjudicate(id);

                if first_result == second_result {
                    // only one resolution!
                    for dep in self.dependencies.drain(old_dep_count..) {
                        self.orders[dep].state = OrderState::UNRESOLVED;
                    }
                    self.orders[id].resolution = first_result;
                    self.orders[id].state = OrderState::RESOLVED;
                    return first_result;
                }

                // we have circular dependencies; use the backup rule
                self.backup_rule(old_dep_count);

                // start over in case backup rule hasn't resolved all orders
                self.resolve(id)
            }
        }
    }

    // this is what we call from resolve() to tell whether an order follows
    //   the equations
    fn adjudicate(&mut self, id: usize) -> bool {
        // the province being adjudicated
        let province = self.orders[id].province.clone();
        match self.orders[id].action.clone() {

            Action::Hold => {
                // a hold order never fails (what would that even mean)
                true
            },

            Action::Move { to, convoyed: _ } => {
                let attack_strength = self.attack_strength(&province);

                // the attack strength (above) needs to be greater than this
                let counter_strength = if self.orders.iter().any(|o|
                        match o.action {
                            Action::Move { to: ref move_to, convoyed } =>
                                province == *move_to && !convoyed,
                            _ => false
                        } && o.province == to) {
                    // head-to-head battle
                    self.defend_strength(&to)
                } else {
                    // no head-to-head battle
                    self.hold_strength(&to)
                };

                // it also needs to be greater than the prevent strength of all
                //   units moving to the same space
                let contesting_orders = self.orders.iter().filter(|o|
                    match o.action {
                        Action::Move { to: ref move_to, convoyed: _ } =>
                            to == *move_to,
                        _ => false
                    } && o.province != province).map(|o| o.province.clone())
                    .collect::<Vec<Province>>();

                // return whether it satisfies both these conditions
                attack_strength > counter_strength && contesting_orders.iter()
                    .all(|p| attack_strength > self.prevent_strength(&p))
            },

            Action::SupportHold { to } | Action::SupportMove { from: _, to } => {
                // a support is cut when...
                !self.orders.clone().iter().any(|o|
                    match o.action {
                        Action::Move { to: ref move_to, convoyed } =>
                            // ... something with a valid path attacks it...
                            province == *move_to && if convoyed {
                                !self.convoy_paths(o).is_empty()
                            } else { true },
                        _ => false
                    } &&
                    // ... and it's not the thing being supported (in)to...
                    o.province != to &&
                    // ... , and you can't cut your own support
                    o.owner != self.orders[id].owner)
            },

            Action::Convoy { from: _, to: _ } => {
                // a convoy only fails when it is dislodged
                !self.orders.clone().iter().any(|o|
                    match o.action {
                        Action::Move { to: ref move_to, convoyed: _ } => {
                            province == *move_to
                        },
                        _ => false
                    } && self.resolve(o.id))
            },

        }
    }

    // this returns all valid paths a convoyed army can go through to get to
    //   its destination, taking into account dislodged fleets
    fn convoy_paths(&mut self, order: &Order) -> Vec<Vec<Province>> {
        match order.action {
            Action::Move { ref to, convoyed } => { if convoyed {

                // first, find all paths at all through water that get from
                //   the province of the order to the destination
                let paths: Vec<Vec<Province>> = self.find_paths(
                    vec![self.get_region(&order.province).unwrap()], to)
                    .iter().map(|path|
                        path.iter().map(|r| r.province.clone()).collect()
                    ).collect();

                // now filter those paths for the ones that are actually valid
                paths.iter().filter(|path| {
                    path.iter().skip(1).all(|&ref p|
                        // for every convoying fleet...
                        self.orders.clone().iter().any(|o|
                            o.province == *p && match o.action {
                                // ... there has to be a convoy order
                                Action::Convoy { ref from, to: ref c_to } => {
                                    *from == order.province && *to == *c_to
                                }, _ => false
                            } && self.resolve(o.id)  // ... and it must succeed
                        )
                    )
                }).map(|x|x.clone()).collect()

            } else { panic!("convoy_paths called on non-convoyed Move"); } },
            _ => panic!("convoy_paths called on non-Move")
        }
    }

    // utility function used from convoy_paths (see above)
    fn find_paths<'a>(&'a self, path: Vec<&'a MapRegion>, target: &Province)
            -> Vec<Vec<&MapRegion>> {
        // the "end" of the current chain
        let region = path.last().unwrap().clone();
        // if we've made it already, return
        if region.fleet_borders.contains(target) { return vec![path]; }
        // otherwise, find the next fleet in the chain
        self.map.iter().filter(|&r|
                // it's empty water if we can move to it as a fleet but can't
                //   move to it as an army
                region.fleet_borders.contains(&r.province) &&
                !region.army_borders.contains(&r.province) &&
                // we also need to make sure we don't get in an infinite loop
                !path.contains(&&r)).flat_map(|r| {
                    // add the next fleet to the path
                    let mut new_path = path.clone();
                    new_path.push(&r);
                    // and recurse
                    self.find_paths(new_path, target)
                }).collect()
    }

    fn hold_strength(&mut self, province: &Province) -> usize {
        if self.get_unit(province).is_some() {
            // figure out if the unit in this region is moving away
            let move_id = self.orders.iter().find(|o|
                match o.action {
                    Action::Move { to: _, convoyed: _ } => true, _ => false
                } && o.province == *province).map(|o| o.id);

            if let Some(move_id) = move_id {
                // if the unit moves away successfully, we treat the province
                //   as empty. otherwise, it always has hold strength of 1,
                //   regardless of support
                if self.resolve(move_id) { 0 } else { 1 }
            } else {
                // hold strength is 1 plus the number of successful orders to
                //   support hold
                1 + self.orders.clone().iter().filter(|o|
                    match o.action {
                        Action::SupportHold { ref to } => *to == *province,
                        _ => false
                    } && self.resolve(o.id)).count()
            }
        } else {
            // the hold strength of an empty province is always 0
            0
        }
    }

    fn attack_strength(&mut self, province: &Province) -> usize {
        // first, if there's no move order, attack strength doesn't make sense
        // otherwise, use it to find the destination and whether it's a convoy
        let move_order = if let Some(move_order) = self.orders.iter().find(|o|
                match o.action {
                    Action::Move { to: _, convoyed: _ } => true, _ => false
                } && o.province == *province) { move_order }
            else { panic!("attack_strength called on non-Move"); }.clone();
        let (dest, convoyed) = match move_order.action {
            Action::Move { ref to, convoyed } => (to, convoyed),
            _ => unreachable!()
        };

        // attack strength is 0 if the path is invalid
        if convoyed && self.convoy_paths(&move_order).is_empty() { return 0; }

        // now we check to see whether the unit at the destination has moved
        //   away, given that it's not a head-to-head battle. this is important
        //   because we cannot call resolve if it is one, as that would cause
        //   the recursion to become infinite
        let move_id = self.orders.iter().find(|o|
            match o.action {
                Action::Move { ref to, convoyed: _ } => *to != *province,
                _ => false
            } && o.province == *dest).map(|o| o.id);
        let moved_away = move_id.map_or(false, |id| self.resolve(id));

        // we also figure out which power we're attacking
        let attacked_power = if moved_away {
            None
        } else {
            self.get_region(dest)
                .and_then(|r| r.clone().unit.map(|u| u.owner.clone()))
        };

        // because if we attack ourselves, attack strength is always 0
        if attacked_power == Some(move_order.owner) { return 0; }

        // otherwise, attack strength is 1 plus the number of successful orders
        //   to support the move
        let supports: Vec<usize> = self.orders.iter().filter(|o|
            match o.action {
                Action::SupportMove { ref from, ref to } =>
                    *from == *province && *to == *dest,
                _ => false
            } &&
            attacked_power.as_ref().map_or(true, |power| *power != o.owner))
            .map(|o| o.id).collect();

        1 + supports.iter().filter(|&id| self.resolve(*id)).count()
    }

    fn defend_strength(&mut self, province: &Province) -> usize {
        // similar to attack strength, first find the move in question
        let move_order = if let Some(move_order) = self.orders.iter().find(|o|
                match o.action {
                    Action::Move { to: _, convoyed: _ } => true, _ => false
                } && o.province == *province) { move_order }
            else { panic!("defend_strength called on non-Move"); }.clone();
        let dest = match move_order.action {
            Action::Move { ref to, convoyed: _ } => to,
            _ => unreachable!()
        };

        // defend strength is just 1 plus number of successful support moves
        let supports: Vec<usize> = self.orders.iter().filter(|o|
            match o.action {
                Action::SupportMove { ref from, ref to } =>
                    *from == *province && *to == *dest,
                _ => false
            }).map(|o| o.id).collect();

        1 + supports.iter().filter(|&id| self.resolve(*id)).count()
    }

    fn prevent_strength(&mut self, province: &Province) -> usize {
        // same as always...
        let move_order = if let Some(move_order) = self.orders.iter().find(|o|
                match o.action {
                    Action::Move { to: _, convoyed: _ } => true, _ => false
                } && o.province == *province) { move_order }
            else { panic!("prevent_strength called on non-Move"); }.clone();
        let (dest, convoyed) = match move_order.action {
            Action::Move { ref to, convoyed } => (to, convoyed),
            _ => unreachable!()
        };

        // prevent strength also requires a successful path in case of convoy
        if convoyed && self.convoy_paths(&move_order).is_empty() { return 0; }

        // if we're in a head-to-head battle and lose, prevent strength is 0
        let move_id = self.orders.iter().find(|o|
            match o.action {
                Action::Move { ref to, convoyed: _ } => *to == *province,
                _ => false
            } && o.province == *dest).map(|o| o.id);
        if let Some(move_id) = move_id {
            if self.resolve(move_id) { return 0; }
        }

        // otherwise, 1 plus number of successful support moves
        let supports: Vec<usize> = self.orders.iter().filter(|o|
            match o.action {
                Action::SupportMove { ref from, ref to } =>
                    *from == *province && *to == *dest,
                _ => false
            }).map(|o| o.id).collect();

        1 + supports.iter().filter(|&id| self.resolve(*id)).count()
    }

    fn backup_rule(&mut self, old_dep_count: usize) {
        let dependencies = self.dependencies.drain(old_dep_count..)
            .collect::<Vec<usize>>();
        let (mut only_moves, mut convoys) = (true, false);

        for &dep in dependencies.iter() {
            match self.orders[dep].action {
                Action::Move { to: _, convoyed: _ } => {},
                Action::Convoy { from: _, to: _ } => {
                    only_moves = false;
                    convoys = true;
                },
                _ => only_moves = false
            }
        }

        for &dep in dependencies.iter() {
            if only_moves {
                // circular movement---make everything succeed
                self.orders[dep].resolution = true;
                self.orders[dep].state = OrderState::RESOLVED;
            } else if convoys {
                // convoy paradox---make convoy fail as per Szykman
                let is_convoy = match self.orders[dep].action {
                    Action::Convoy { from: _, to: _ } => true, _ => false
                };
                if is_convoy {
                    self.orders[dep].resolution = false;
                    self.orders[dep].state = OrderState::RESOLVED;
                } else {
                    self.orders[dep].state = OrderState::UNRESOLVED;
                }
            } else {
                panic!("unknown circular dependency");
            }
        }
    }
}
