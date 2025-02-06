use stpsyr::types::*;

extern crate bincode;

impl Stpsyr {
    // parse orders as a string and apply them
    pub fn parse(&mut self, power: &Power, orders: String) {
        match self.phase {
            Phase::SpringDiplomacy | Phase::FallDiplomacy => self.parse_orders(power, orders),
            Phase::SpringRetreats | Phase::FallRetreats => self.parse_retreats(power, orders),
            Phase::Builds => self.parse_adjusts(power, orders),
        }
    }

    pub fn apply(&mut self) {
        match self.phase {
            Phase::SpringDiplomacy | Phase::FallDiplomacy => self.apply_orders(),
            Phase::SpringRetreats | Phase::FallRetreats => self.apply_retreats(),
            Phase::Builds => self.apply_adjusts(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(encoded: &[u8]) {
        bincode::deserialize(encoded).unwrap()
    }

    fn parse_orders(&mut self, power: &Power, orders: String) {
        for line in orders.lines() {
            let line = line.to_lowercase().replace('(', "/").replace(" /", "/");
            let tokens: Vec<&str> = line
                .split(|c: char| !(c.is_lowercase() || c == '/'))
                .collect();
            let mut tokens_iter = tokens
                .iter()
                .filter(|&token| {
                    (token.len() >= 3 || *token == "s" || *token == "c" || *token == "vc")
                        && *token != "army"
                        && *token != "fleet"
                        && *token != "hold"
                        && *token != "holds"
                        && *token != "stand"
                        && *token != "stands"
                        && *token != "move"
                        && *token != "moves"
                        && *token != "the"
                        && *token != "coast"
                        && *token != "via"
                })
                .map(|&token| match token {
                    "support" | "supports" => "s",
                    "convoy" | "convoys" | "vc" => "c",
                    _ => token,
                });

            let province = if let Some(p) = tokens_iter.next() {
                Province::from(p)
            } else {
                continue;
            };

            match tokens_iter.next() {
                None => {} // hold
                Some(token2) => {
                    match token2 {
                        "s" => {
                            // support
                            let a = tokens_iter.next().unwrap();
                            if let Some(b) = tokens_iter.next() {
                                // support move
                                self.add_order(
                                    power.clone(),
                                    province,
                                    Action::SupportMove {
                                        from: Province::from(a),
                                        to: Province::from(b),
                                    },
                                );
                            } else {
                                // support hold
                                self.add_order(
                                    power.clone(),
                                    province,
                                    Action::SupportHold {
                                        to: Province::from(a),
                                    },
                                );
                            }
                        }
                        "c" => {
                            // convoy
                            let from = tokens_iter.next().unwrap();
                            let to = tokens_iter.next().unwrap();
                            self.add_order(
                                power.clone(),
                                province,
                                Action::Convoy {
                                    from: Province::from(from),
                                    to: Province::from(to),
                                },
                            );
                        }
                        _ => {
                            // regular move
                            let vc = tokens_iter.next() == Some("c");
                            self.add_order(
                                power.clone(),
                                province,
                                Action::Move {
                                    to: Province::from(token2),
                                    convoyed: vc,
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    fn parse_retreats(&mut self, power: &Power, orders: String) {
        for line in orders.lines() {
            let line = line
                .to_lowercase()
                .replace('(', "/")
                .replace(" /", "/")
                .replace("/ ", "/");
            let tokens: Vec<&str> = line
                .split(|c: char| !(c.is_lowercase() || c == '/'))
                .collect();
            let mut tokens_iter = tokens.iter().filter(|&token| {
                token.len() >= 3
                    && *token != "army"
                    && *token != "fleet"
                    && *token != "move"
                    && *token != "moves"
                    && *token != "retreat"
                    && *token != "retreats"
                    && *token != "disband"
                    && *token != "disbands"
                    && *token != "the"
                    && *token != "coast"
            });

            let p1 = if let Some(p) = tokens_iter.next() {
                Province::from(*p)
            } else {
                continue;
            };

            if let Some(p2) = tokens_iter.next() {
                if tokens_iter.next().is_some() {
                    continue;
                }
                self.add_retreat(
                    power.clone(),
                    p1,
                    RetreatAction::Move {
                        to: Province::from(*p2),
                    },
                );
            } else {
                self.add_retreat(power.clone(), p1, RetreatAction::Disband);
            }
        }
    }

    fn parse_adjusts(&mut self, power: &Power, orders: String) {
        for line in orders.lines() {
            let line = line
                .to_lowercase()
                .replace('(', "/")
                .replace(" /", "/")
                .replace("/ ", "/");
            let tokens: Vec<&str> = line
                .split(|c: char| !(c.is_lowercase() || c == '/'))
                .collect();
            let mut tokens_iter = tokens
                .iter()
                .filter(|&token| {
                    (token.len() >= 3 || *token == "d" || *token == "a" || *token == "f")
                        && *token != "build"
                        && *token != "the"
                        && *token != "coast"
                })
                .map(|&token| match token {
                    "destroy" => "d",
                    "army" => "a",
                    "fleet" => "f",
                    _ => token,
                });

            match tokens_iter.next() {
                Some("d") => {
                    if let Some(p) = tokens_iter.next() {
                        self.add_adjust(power.clone(), Province::from(p), AdjustAction::Disband);
                    }
                }
                Some("a") => {
                    if let Some(p) = tokens_iter.next() {
                        self.add_adjust(
                            power.clone(),
                            Province::from(p),
                            AdjustAction::Build {
                                unit_type: UnitType::Army,
                            },
                        );
                    }
                }
                Some("f") => {
                    if let Some(p) = tokens_iter.next() {
                        self.add_adjust(
                            power.clone(),
                            Province::from(p),
                            AdjustAction::Build {
                                unit_type: UnitType::Fleet,
                            },
                        );
                    }
                }
                _ => {} // invalid
            }
        }
    }
}
