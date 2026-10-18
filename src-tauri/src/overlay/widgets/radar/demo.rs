//! Follows a demo file while TF2 is still recording it and keeps track of your soul gargoyle.
//!
//! TF2 writes client demos through a 16 KB buffer, so the file trails the game by up to 16 KB of data.
//! That is fine here: a gargoyle does not move during its 4-minute life.
//!
//! The parsing state from tf-demo-parser owns all its data, so each newly appended chunk is parsed with a
//! short-lived buffer; a packet that is only half written stays in `pending` until the rest arrives.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tf_demo_parser::demo::header::Header;
use tf_demo_parser::demo::message::packetentities::{PacketEntity, UpdateType};
use tf_demo_parser::demo::message::{Message, MessageType};
use tf_demo_parser::demo::packet::stringtable::StringTable;
use tf_demo_parser::demo::packet::Packet;
use tf_demo_parser::demo::parser::RawPacketStream;
use tf_demo_parser::demo::sendprop::{SendPropIdentifier, SendPropValue};
use tf_demo_parser::demo::vector::Vector;
use tf_demo_parser::{Demo, ParserState};

const HEADER_BYTES: usize = 1072;
const GIFT_CLASS: &str = "CHalloweenGiftPickup";
const ORIGIN: SendPropIdentifier = SendPropIdentifier::new("DT_BaseEntity", "m_vecOrigin");
const TARGET: SendPropIdentifier =
    SendPropIdentifier::new("DT_CHalloweenGiftPickup", "m_hTargetPlayer");
/// Networked entity handles carry the entity index in their low 11 bits (MAX_EDICT_BITS).
const HANDLE_INDEX_MASK: i64 = (1 << 11) - 1;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Gift {
    origin: Option<[f32; 3]>,
    target: Option<u32>,
    /// The demo tick it was created on; None if it already existed when the recording started.
    spawned_tick: Option<u32>,
}

pub struct DemoFollower {
    path: PathBuf,
    class_name: &'static str,
    /// Follow any gargoyle, not just yours (for testing with `ent_create tf_halloween_gift_pickup`).
    pub any_owner: bool,
    offset: u64,
    pending: Vec<u8>,
    header_read: bool,
    state: ParserState,
    table_names: Vec<String>,
    gift_class: Option<u16>,
    local_entity: Option<u32>,
    gifts: HashMap<u32, Gift>,
    /// The tick of the newest gameplay packet read, and the length of a tick in seconds.
    tick: u32,
    interval_per_tick: f32,
    pub map: Option<String>,
    /// TF2 wrote the stop packet: the recording ended (disconnect, or a new `record`/`stop`).
    pub ended: bool,
}

fn parse_entities_only(t: MessageType) -> bool {
    t == MessageType::PacketEntities
}

impl DemoFollower {
    pub fn new(path: &Path) -> Self {
        Self::tracking(path, GIFT_CLASS)
    }

    /// Follows entities of another class (tests use classes that appear in ordinary demos).
    fn tracking(path: &Path, class_name: &'static str) -> Self {
        DemoFollower {
            path: path.to_path_buf(),
            class_name,
            any_owner: false,
            offset: 0,
            pending: Vec::new(),
            header_read: false,
            state: ParserState::new(24, parse_entities_only, false),
            table_names: Vec::new(),
            gift_class: None,
            local_entity: None,
            gifts: HashMap::new(),
            tick: 0,
            interval_per_tick: 0.015,
            map: None,
            ended: false,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes_read(&self) -> u64 {
        self.offset
    }

    /// Your gargoyle's position, if one currently exists for you.
    pub fn my_gargoyle(&self) -> Option<[f32; 3]> {
        self.mine().and_then(|g| g.origin)
    }

    /// How long ago your gargoyle spawned, as of the newest data read; None if it spawned before the recording.
    pub fn my_gargoyle_age(&self) -> Option<Duration> {
        let spawned = self.mine()?.spawned_tick?;
        let ticks = self.tick.saturating_sub(spawned);
        Duration::try_from_secs_f32(ticks as f32 * self.interval_per_tick).ok()
    }

    fn mine(&self) -> Option<&Gift> {
        if self.ended {
            return None;
        }
        if self.any_owner {
            return self.gifts.values().find(|g| g.origin.is_some());
        }
        let me = self.local_entity?;
        self.gifts
            .values()
            .find(|g| g.target == Some(me) && g.origin.is_some())
    }

    /// Reads whatever TF2 has appended since the last call. Returns false if the file shrank or vanished
    /// (a new recording replaced it), in which case the caller should start over.
    pub fn poll(&mut self) -> bool {
        let Ok(mut file) = File::open(&self.path) else {
            return false;
        };
        let Ok(len) = file.metadata().map(|m| m.len()) else {
            return false;
        };
        if len < self.offset {
            return false;
        }
        // TF2 never appends to a recording after its stop packet, so growth means a new one replaced it
        if self.ended && len > self.offset {
            return false;
        }
        if len > self.offset {
            let mut chunk = Vec::with_capacity((len - self.offset) as usize);
            if file.seek(SeekFrom::Start(self.offset)).is_err()
                || file.read_to_end(&mut chunk).is_err()
            {
                return true;
            }
            self.offset += chunk.len() as u64;
            self.pending.extend_from_slice(&chunk);
            self.consume();
        }
        true
    }

    fn consume(&mut self) {
        if !self.header_read {
            if self.pending.len() < HEADER_BYTES {
                return;
            }
            let demo = Demo::new(&self.pending);
            let mut stream = demo.get_stream();
            if let Ok(header) = stream.read::<Header>() {
                self.state.protocol_version = header.protocol;
            }
            self.header_read = true;
            self.pending.drain(..HEADER_BYTES);
        }

        let buffer = std::mem::take(&mut self.pending);
        let mut packets = RawPacketStream::new(Demo::new(&buffer).get_stream());
        let mut used_bits = 0;
        loop {
            match packets.next(&self.state) {
                Ok(Some(packet)) => {
                    used_bits = packets.pos();
                    self.handle_packet(packet);
                }
                // None: the next packet is not fully written yet. Err: corrupt data; skip what we have.
                Ok(None) => break,
                Err(_) => {
                    used_bits = buffer.len() * 8;
                    break;
                }
            }
        }
        // packets start on byte boundaries, so a packet's end is too
        self.pending = buffer[(used_bits / 8).min(buffer.len())..].to_vec();
    }

    fn handle_packet(&mut self, packet: Packet) {
        // sign-on ticks run ahead of the gameplay ticks that follow them, so only gameplay packets move the clock
        if let Packet::Message(p) = &packet {
            self.tick = u32::from(p.tick);
        }
        match packet {
            Packet::DataTables(p) => {
                if self
                    .state
                    .handle_data_table(&p.tables, p.server_classes)
                    .is_ok()
                {
                    self.gift_class = self
                        .state
                        .server_classes
                        .iter()
                        .find(|c| c.name.as_str() == self.class_name)
                        .map(|c| u16::from(c.id));
                }
            }
            Packet::StringTables(p) => {
                for table in p.tables {
                    self.handle_string_table(table);
                }
            }
            Packet::Message(p) | Packet::Signon(p) => {
                let tick = u32::from(p.tick);
                for message in p.messages {
                    match message {
                        Message::CreateStringTable(m) => self.handle_string_table(m.table),
                        Message::UpdateStringTable(m) => {
                            if let Some(name) = self.table_names.get(m.table_id as usize) {
                                for (index, entry) in &m.entries {
                                    self.state.handle_string_entry(name, *index as usize, entry);
                                }
                            }
                        }
                        Message::ServerInfo(info) => {
                            // a new level: everything entity-related starts over
                            self.local_entity = Some(info.player_slot as u32 + 1);
                            self.interval_per_tick = info.interval_per_tick;
                            self.map = Some(info.map.clone());
                            self.gifts.clear();
                            self.state.handle_message(Message::ServerInfo(info), p.tick);
                        }
                        Message::PacketEntities(m) => {
                            // read gifts before the state applies the update (Enter props need the old baselines)
                            // a full update (no delta) lists what already exists; only a delta shows new entities
                            let spawned_tick = m.delta.is_some().then_some(tick);
                            for entity in &m.entities {
                                self.track_gift(entity, spawned_tick);
                            }
                            for removed in &m.removed_entities {
                                self.gifts.remove(&u32::from(*removed));
                            }
                            self.state
                                .handle_message(Message::PacketEntities(m), p.tick);
                        }
                        other => self.state.handle_message(other, p.tick),
                    }
                }
            }
            Packet::Stop(_) => {
                self.ended = true;
                self.gifts.clear();
            }
            _ => {}
        }
    }

    fn handle_string_table(&mut self, table: StringTable) {
        self.state.handle_string_table_meta(table.get_table_meta());
        for (index, entry) in &table.entries {
            self.state
                .handle_string_entry(&table.name, *index as usize, entry);
        }
        self.table_names.push(table.name.into_owned());
    }

    fn track_gift(&mut self, entity: &PacketEntity, spawned_tick: Option<u32>) {
        let index = u32::from(entity.entity_index);
        if Some(u16::from(entity.server_class)) != self.gift_class {
            return;
        }
        if matches!(entity.update_type, UpdateType::Leave | UpdateType::Delete) {
            if entity.update_type == UpdateType::Delete {
                self.gifts.remove(&index);
            }
            return;
        }
        let gift = self.gifts.entry(index).or_insert_with(|| Gift {
            spawned_tick: spawned_tick.filter(|_| entity.update_type == UpdateType::Enter),
            ..Gift::default()
        });
        for prop in entity.props(&self.state) {
            match (prop.identifier, &prop.value) {
                (ORIGIN, SendPropValue::Vector(Vector { x, y, z })) => {
                    gift.origin = Some([*x, *y, *z])
                }
                (TARGET, value) => {
                    gift.target = i64::try_from(value).ok().and_then(target_index);
                }
                _ => {}
            }
        }
    }
}

/// Entity index from a networked handle, or None for the "no entity" handle.
fn target_index(handle: i64) -> Option<u32> {
    let index = handle & HANDLE_INDEX_MASK;
    (index != 0 && index != HANDLE_INDEX_MASK).then_some(index as u32)
}

/// Finds the demo TF2 is currently writing: the .dem directly in `tf` or `tf\demos` that grew most recently.
///
/// Sizes come from opening each file: for a file still open for writing, Windows does not update the size
/// or modified time in the folder listing until it is closed, so the listing makes a live recording look old.
#[derive(Default)]
pub struct DemoFinder {
    seen: HashMap<PathBuf, (u64, Instant)>,
}

impl DemoFinder {
    pub fn active(&mut self, tf_dir: &Path, fresh: Duration) -> Option<PathBuf> {
        let now = Instant::now();
        let files: Vec<PathBuf> = [tf_dir.to_path_buf(), tf_dir.join("demos")]
            .iter()
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flatten()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("dem")))
            .collect();
        self.seen.retain(|path, _| files.contains(path));
        for path in files {
            let Some(size) = File::open(&path)
                .and_then(|f| f.metadata())
                .ok()
                .map(|m| m.len())
            else {
                continue;
            };
            match self.seen.get_mut(&path) {
                Some(entry) if entry.0 == size => {}
                Some(entry) => *entry = (size, now),
                // first sighting: not known to be growing yet
                None => {
                    self.seen.insert(path, (size, now - fresh));
                }
            }
        }
        self.seen
            .iter()
            .filter(|(_, (_, changed))| now.duration_since(*changed) < fresh)
            .max_by_key(|(_, (_, changed))| *changed)
            .map(|(path, _)| path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real TF2 demo from the tf-demo-parser repository (test_data/short-2024.dem, MIT/Apache-2.0).
    fn sample_demo() -> Option<Vec<u8>> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/testdata/short-2024.dem"
        ))
        .ok()
    }

    /// Writes `data` to a fresh file in steps (as TF2 would while recording) and polls a follower after each.
    /// Also returns the most entities tracked with a known origin at any point, and every spawn tick seen.
    fn follow_in_chunks(
        data: &[u8],
        class: &'static str,
        chunk_sizes: &mut dyn FnMut() -> usize,
    ) -> (DemoFollower, usize, Vec<u32>) {
        static RUN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let run = RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "radar_demo_test_{}_{run}_{class}.dem",
            std::process::id()
        ));
        std::fs::write(&path, []).unwrap();
        let mut follower = DemoFollower::tracking(&path, class);
        let (mut written, mut most, mut spawns) = (0, 0, Vec::new());
        while written < data.len() {
            let end = (written + chunk_sizes()).min(data.len());
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            std::io::Write::write_all(&mut file, &data[written..end]).unwrap();
            drop(file);
            written = end;
            assert!(follower.poll());
            most = most.max(
                follower
                    .gifts
                    .values()
                    .filter(|g| g.origin.is_some())
                    .count(),
            );
            spawns.extend(follower.gifts.values().filter_map(|g| g.spawned_tick));
        }
        std::fs::remove_file(&path).unwrap();
        (follower, most, spawns)
    }

    #[test]
    fn streaming_in_chunks_matches_reading_the_whole_demo() {
        let Some(data) = sample_demo() else {
            eprintln!("skipped: testdata/short-2024.dem not present");
            return;
        };
        // the recording's final stop packet clears everything, so compare the state just before it
        let (full, data) = (&data[..], &data[..data.len() - 16]);
        let mut tried = Vec::new();
        // classes present in this clip; doors also move, which exercises delta updates
        for class in ["CDynamicProp", "CBaseDoor", "CLightGlow"] {
            let (whole, _, _) = follow_in_chunks(data, class, &mut || usize::MAX);
            assert!(
                whole.map.is_some() && whole.local_entity.is_some(),
                "server info not read"
            );
            // uneven chunk sizes from 1 KB to 40 KB, so packets get split at arbitrary points
            let mut n = 7u64;
            let (chunked, most, _) = follow_in_chunks(data, class, &mut || {
                n = n
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                1024 + (n >> 33) as usize % (40 * 1024)
            });
            assert_eq!(
                whole.gifts, chunked.gifts,
                "{class}: chunked parsing diverged"
            );
            assert_eq!(whole.bytes_read(), data.len() as u64);
            assert!(!whole.ended, "{class}: ended before the stop packet");
            tried.push((class, whole.gift_class.is_some(), most));
        }
        eprintln!(
            "map {:?}; classes (name, present, most tracked at once): {tried:?}",
            follow_in_chunks(data, "x", &mut || usize::MAX).0.map
        );
        assert!(
            tried.iter().any(|(_, _, most)| *most > 0),
            "no entity of any test class was ever tracked"
        );

        // reading the whole recording, stop packet included, ends tracking
        let (ended, _, _) = follow_in_chunks(full, "CDynamicProp", &mut || 4096);
        assert!(ended.ended && ended.gifts.is_empty());
    }

    #[test]
    fn spawn_ticks_only_for_entities_created_while_recording() {
        let Some(data) = sample_demo() else {
            eprintln!("skipped: testdata/short-2024.dem not present");
            return;
        };
        let data = &data[..data.len() - 16];
        // props exist when the recording starts; the clip's one scene entity is created three ticks in
        let (props, _, spawns) = follow_in_chunks(data, "CDynamicProp", &mut || 4096);
        assert!(!props.gifts.is_empty() && spawns.is_empty());
        let (_, _, spawns) = follow_in_chunks(data, "CSceneEntity", &mut || 4096);
        assert_eq!(spawns.first(), Some(&3));
    }

    #[test]
    fn clock_ends_on_the_gameplay_tick_not_the_sign_on_tick() {
        let Some(data) = sample_demo() else {
            eprintln!("skipped: testdata/short-2024.dem not present");
            return;
        };
        let data = &data[..data.len() - 16];
        // sign-on packets reach tick ~2530 before gameplay starts near 0
        let (scene, _, _) = follow_in_chunks(data, "CSceneEntity", &mut || 4096);
        assert!(scene.tick > 3 && scene.tick < 1000, "tick {}", scene.tick);
    }

    #[test]
    fn growth_after_the_stop_packet_means_a_new_recording() {
        let Some(data) = sample_demo() else {
            eprintln!("skipped: testdata/short-2024.dem not present");
            return;
        };
        let path =
            std::env::temp_dir().join(format!("radar_replaced_test_{}.dem", std::process::id()));
        std::fs::write(&path, &data).unwrap();
        let mut follower = DemoFollower::new(&path);
        assert!(follower.poll());
        assert!(follower.ended);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(&mut file, &[0u8; 16]).unwrap();
        drop(file);
        let replaced = !follower.poll();
        std::fs::remove_file(&path).unwrap();
        assert!(
            replaced,
            "a file that grew after its stop packet was not reported"
        );
    }

    #[test]
    fn finder_picks_the_file_that_is_growing() {
        let dir = std::env::temp_dir().join(format!("radar_finder_test_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("demos")).unwrap();
        let (old, live) = (dir.join("old.dem"), dir.join("demos").join("live.dem"));
        std::fs::write(&old, [1u8; 10]).unwrap();
        std::fs::write(&live, [1u8; 10]).unwrap();
        let fresh = Duration::from_millis(300);
        let mut finder = DemoFinder::default();
        assert_eq!(finder.active(&dir, fresh), None, "nothing has grown yet");
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&live)
            .unwrap();
        std::io::Write::write_all(&mut f, &[2u8; 10]).unwrap();
        assert_eq!(finder.active(&dir, fresh), Some(live.clone()));
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(
            finder.active(&dir, fresh),
            None,
            "stopped growing: no longer active"
        );
        drop(f);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn handles_map_to_entity_indexes() {
        // index 5, serial 3 in the upper bits
        assert_eq!(target_index(5 | (3 << 11)), Some(5));
        // the invalid handle (all 21 bits set) and index 0 mean "nobody"
        assert_eq!(target_index((1 << 21) - 1), None);
        assert_eq!(target_index(0), None);
    }
}
