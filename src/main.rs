/*
 * stpsyr - a Diplomacy adjudicator in Rust
 * Copyright (C) 2017  Keyboard Fire <andy@keyboardfire.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

extern crate stpsyr;
use stpsyr::*;
extern crate serde_json;
fn main() {
    let mut s = Stpsyr::new("data/standard.csv");
    s.parse(&Power::from("Italy"), "A ven-tyr".to_string());
    s.apply();
    let json_value = serde_json::to_string(&s).unwrap();
    s.render_to_file("meems.svg".to_string());
    println!("{}", json_value);
}
