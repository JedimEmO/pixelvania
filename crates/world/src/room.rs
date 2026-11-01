//! The text room format.
//!
//! A room is a plain text file in three sections. `[legend]` names the tile characters,
//! `[tiles]` is the room drawn in those characters, one line per row of 16 x 16 tiles,
//! and `[entities]` places things in the room. Lines starting with `//` are comments.
//!
//! ```text
//! [room]
//! name = lab
//!
//! [legend]
//! # stone
//!
//! [tiles]
//! ##########
//! #........#
//! #........#
//! ##########
//!
//! [entities]
//! spawn 2 2
//! light 5 1.5 radius=96 color=ffc070 strength=1.2
//! ```
//!
//! `.` and space are always empty. Every other tile character must appear in the legend.
//! Entity positions are in tiles from the top-left corner of the room, and may be fractional.
//! Everything after the position is a `key=value` property.

use std::collections::BTreeMap;
use std::fmt;

/// The tile material the Plunge breaks.
pub const CRACKED: &str = "cracked";

/// The kinds of entity the Lure can take hold of: lights that hang.
pub const LURE_KINDS: [&str; 3] = ["chandelier", "lantern", "bloom"];

/// Width and height of one tile, in pixels.
pub const TILE: i32 = 16;

/// A parsed room.
#[derive(Debug, Clone, PartialEq)]
pub struct Room {
    pub name: String,
    /// Optional backdrop generator name from `[room]`.
    pub background: Option<String>,
    /// Optional music theme from `[room]`.
    pub music: Option<String>,
    /// Width in tiles.
    pub width: usize,
    /// Height in tiles.
    pub height: usize,
    /// Material names, indexed by [`Tile::Solid`].
    pub materials: Vec<String>,
    /// Row-major, `width * height` tiles.
    pub tiles: Vec<Tile>,
    pub entities: Vec<EntityDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tile {
    Empty,
    /// A solid tile of the material at this index in [`Room::materials`].
    Solid(u8),
}

/// One line of the `[entities]` section.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityDef {
    pub kind: String,
    /// Position in tiles from the room's top-left corner.
    pub x: f32,
    pub y: f32,
    pub props: BTreeMap<String, String>,
    /// The line in the source file, for error messages.
    pub line: usize,
}

impl EntityDef {
    /// Position in pixels from the room's top-left corner, y down.
    pub fn pixel_pos(&self) -> (f32, f32) {
        (self.x * TILE as f32, self.y * TILE as f32)
    }

    /// Reads a numeric property, or `default` when it is absent.
    pub fn num(&self, key: &str, default: f32) -> Result<f32, ParseError> {
        match self.props.get(key) {
            None => Ok(default),
            Some(v) => v.parse().map_err(|_| ParseError {
                line: self.line,
                message: format!("`{key}={v}` is not a number"),
            }),
        }
    }

    /// Reads a `rrggbb` hex colour property as 0..1 floats, or `default` when it is absent.
    pub fn color(&self, key: &str, default: [f32; 3]) -> Result<[f32; 3], ParseError> {
        let Some(v) = self.props.get(key) else {
            return Ok(default);
        };
        let hex = v.trim_start_matches('#');
        let bad = || ParseError {
            line: self.line,
            message: format!("`{key}={v}` is not an rrggbb colour"),
        };
        if hex.len() != 6 {
            return Err(bad());
        }
        let n = u32::from_str_radix(hex, 16).map_err(|_| bad())?;
        Ok([
            ((n >> 16) & 255) as f32 / 255.0,
            ((n >> 8) & 255) as f32 / 255.0,
            (n & 255) as f32 / 255.0,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based line number.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Room,
    Legend,
    Tiles,
    Entities,
}

impl Room {
    /// Parses a room from the text format described in the module docs.
    pub fn parse(src: &str) -> Result<Room, ParseError> {
        let err = |line: usize, message: String| ParseError { line, message };
        let mut section = Section::None;
        let mut name = String::new();
        let mut background = None;
        let mut music = None;
        let mut legend: BTreeMap<char, u8> = BTreeMap::new();
        let mut materials: Vec<String> = Vec::new();
        let mut rows: Vec<(usize, &str)> = Vec::new();
        let mut entities = Vec::new();

        for (i, raw) in src.lines().enumerate() {
            let line_no = i + 1;
            // Tile rows keep their spacing; everything else is trimmed.
            let trimmed = raw.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = match trimmed {
                    "[room]" => Section::Room,
                    "[legend]" => Section::Legend,
                    "[tiles]" => Section::Tiles,
                    "[entities]" => Section::Entities,
                    other => return Err(err(line_no, format!("unknown section `{other}`"))),
                };
                continue;
            }
            match section {
                Section::None => {
                    if !trimmed.is_empty() {
                        return Err(err(line_no, "text before the first section".into()));
                    }
                }
                Section::Room => {
                    if trimmed.is_empty() {
                        continue;
                    }
                    let (key, value) = trimmed
                        .split_once('=')
                        .ok_or_else(|| err(line_no, "expected `key = value`".into()))?;
                    match key.trim() {
                        "name" => name = value.trim().to_owned(),
                        "background" => background = Some(value.trim().to_owned()),
                        "music" => music = Some(value.trim().to_owned()),
                        other => return Err(err(line_no, format!("unknown room key `{other}`"))),
                    }
                }
                Section::Legend => {
                    if trimmed.is_empty() {
                        continue;
                    }
                    let mut chars = trimmed.chars();
                    let c = chars.next().expect("line is not empty");
                    let material = chars.as_str().trim();
                    if c == '.' || c == ' ' {
                        return Err(err(line_no, format!("`{c}` is always empty")));
                    }
                    if material.is_empty() {
                        return Err(err(line_no, format!("`{c}` needs a material name")));
                    }
                    if legend.contains_key(&c) {
                        return Err(err(line_no, format!("`{c}` is defined twice")));
                    }
                    let index = match materials.iter().position(|m| m == material) {
                        Some(i) => i,
                        None => {
                            materials.push(material.to_owned());
                            materials.len() - 1
                        }
                    };
                    let index = u8::try_from(index)
                        .map_err(|_| err(line_no, "too many materials".into()))?;
                    legend.insert(c, index);
                }
                Section::Tiles => {
                    // A blank line ends nothing: rooms may have rows of empty space.
                    // Trailing blank lines are dropped below.
                    rows.push((line_no, raw.trim_end()));
                }
                Section::Entities => {
                    if trimmed.is_empty() {
                        continue;
                    }
                    entities.push(parse_entity(line_no, trimmed)?);
                }
            }
        }

        while rows.last().is_some_and(|(_, r)| r.is_empty()) {
            rows.pop();
        }
        if rows.is_empty() {
            return Err(err(1, "the room has no `[tiles]`".into()));
        }
        let width = rows
            .iter()
            .map(|(_, r)| r.chars().count())
            .max()
            .unwrap_or(0);
        let height = rows.len();
        let mut tiles = Vec::with_capacity(width * height);
        for (line_no, row) in &rows {
            let mut count = 0;
            for c in row.chars() {
                count += 1;
                tiles.push(match c {
                    '.' | ' ' => Tile::Empty,
                    c => Tile::Solid(
                        *legend
                            .get(&c)
                            .ok_or_else(|| err(*line_no, format!("`{c}` is not in the legend")))?,
                    ),
                });
            }
            // Short rows are padded with empty space.
            tiles.extend(std::iter::repeat_n(Tile::Empty, width - count));
        }

        Ok(Room {
            name,
            background,
            music,
            width,
            height,
            materials,
            tiles,
            entities,
        })
    }

    /// The tile at a tile coordinate. Outside the room counts as solid stone, index 0.
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || ty < 0 || tx >= self.width as i32 || ty >= self.height as i32 {
            return Tile::Solid(0);
        }
        self.tiles[ty as usize * self.width + tx as usize]
    }

    /// Empties a tile, as when a floor breaks. Outside the room nothing changes.
    pub fn clear(&mut self, tx: i32, ty: i32) {
        if tx >= 0 && ty >= 0 && tx < self.width as i32 && ty < self.height as i32 {
            self.tiles[ty as usize * self.width + tx as usize] = Tile::Empty;
        }
    }

    /// The name of a tile's material, or `None` for an empty tile.
    pub fn material(&self, tx: i32, ty: i32) -> Option<&str> {
        match self.tile(tx, ty) {
            Tile::Solid(m) => self.materials.get(m as usize).map(String::as_str),
            Tile::Empty => None,
        }
    }

    pub fn is_solid(&self, tx: i32, ty: i32) -> bool {
        self.tile(tx, ty) != Tile::Empty
    }

    /// Size of the room in pixels.
    pub fn pixel_size(&self) -> (i32, i32) {
        (self.width as i32 * TILE, self.height as i32 * TILE)
    }

    /// Where the Lure can take hold, in pixels with y down: every chandelier, lantern and bloom.
    pub fn lure_points(&self) -> Vec<(f32, f32)> {
        self.entities
            .iter()
            .filter(|e| LURE_KINDS.contains(&e.kind.as_str()))
            .map(EntityDef::pixel_pos)
            .collect()
    }

    /// Whether a straight line between two points, in pixels, crosses no solid tile.
    pub fn clear_line(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let steps = (dx.abs().max(dy.abs()) / 4.0).ceil().max(1.0) as i32;
        (0..=steps).all(|i| {
            let k = i as f32 / steps as f32;
            let (x, y) = (from.0 + dx * k, from.1 + dy * k);
            !self.is_solid(
                (x.floor() as i32).div_euclid(TILE),
                (y.floor() as i32).div_euclid(TILE),
            )
        })
    }

    /// Entities of one kind.
    pub fn entities_of<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a EntityDef> + 'a {
        self.entities.iter().filter(move |e| e.kind == kind)
    }
}

fn parse_entity(line_no: usize, line: &str) -> Result<EntityDef, ParseError> {
    let err = |message: String| ParseError {
        line: line_no,
        message,
    };
    let mut parts = line.split_whitespace();
    let kind = parts.next().expect("line is not empty").to_owned();
    let mut coord = |axis: &str| -> Result<f32, ParseError> {
        let v = parts
            .next()
            .ok_or_else(|| err(format!("`{kind}` needs an {axis} position")))?;
        v.parse()
            .map_err(|_| err(format!("`{v}` is not a number for {axis}")))
    };
    let x = coord("x")?;
    let y = coord("y")?;
    let mut props = BTreeMap::new();
    for part in parts {
        let (k, v) = part
            .split_once('=')
            .ok_or_else(|| err(format!("expected `key=value`, found `{part}`")))?;
        props.insert(k.to_owned(), v.to_owned());
    }
    Ok(EntityDef {
        kind,
        x,
        y,
        props,
        line: line_no,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LAB: &str = "\
// A comment.
[room]
name = lab

[legend]
# stone
= wood

[tiles]
#####
#...#
#.=.#
#####

[entities]
spawn 1 1
light 2.5 1 radius=80 color=ff8000
";

    #[test]
    fn background_is_optional_room_metadata() {
        assert_eq!(Room::parse(LAB).unwrap().background, None);
        let room =
            Room::parse(&LAB.replace("name = lab", "name = lab\nbackground = ballroom")).unwrap();
        assert_eq!(room.background.as_deref(), Some("ballroom"));
        assert!(Room::parse(&LAB.replace("name = lab", "unknown = ballroom")).is_err());
    }

    #[test]
    fn music_is_optional_room_metadata() {
        assert_eq!(Room::parse(LAB).unwrap().music, None);
        let room = Room::parse(&LAB.replace("name = lab", "name = lab\nmusic = ballroom")).unwrap();
        assert_eq!(room.music.as_deref(), Some("ballroom"));
        assert_eq!(room.background, None);
    }

    #[test]
    fn parses_a_room() {
        let room = Room::parse(LAB).unwrap();
        assert_eq!(room.name, "lab");
        assert_eq!((room.width, room.height), (5, 4));
        assert_eq!(room.materials, ["stone", "wood"]);
        assert_eq!(room.tile(0, 0), Tile::Solid(0));
        assert_eq!(room.tile(1, 1), Tile::Empty);
        assert_eq!(room.tile(2, 2), Tile::Solid(1));
        assert_eq!(room.entities.len(), 2);
        let light = &room.entities[1];
        assert_eq!(light.kind, "light");
        assert_eq!(light.pixel_pos(), (40.0, 16.0));
        assert_eq!(light.num("radius", 0.0).unwrap(), 80.0);
        assert_eq!(light.num("strength", 1.0).unwrap(), 1.0);
        assert_eq!(
            light.color("color", [0.0; 3]).unwrap(),
            [1.0, 128.0 / 255.0, 0.0]
        );
    }

    #[test]
    fn outside_the_room_is_solid() {
        let room = Room::parse(LAB).unwrap();
        assert!(room.is_solid(-1, 1));
        assert!(room.is_solid(1, 99));
    }

    #[test]
    fn short_rows_are_padded() {
        let room = Room::parse("[legend]\n# stone\n[tiles]\n###\n#\n").unwrap();
        assert_eq!(room.width, 3);
        assert_eq!(room.tile(2, 1), Tile::Empty);
    }

    #[test]
    fn unknown_tiles_are_reported_with_their_line() {
        let e = Room::parse("[legend]\n# stone\n[tiles]\n###\n#X#\n").unwrap_err();
        assert_eq!(e.line, 5);
        assert!(e.message.contains("`X`"));
    }

    #[test]
    fn bad_entity_numbers_are_reported() {
        let e = Room::parse("[tiles]\n...\n[entities]\nspawn one 2\n").unwrap_err();
        assert_eq!(e.line, 4);
    }
}
