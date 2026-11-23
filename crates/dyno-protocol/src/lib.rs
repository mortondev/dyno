//! Dyno network protocol (bootstrap).
//!
//! Every message is one binary WebSocket message, encoded with postcard.
//! WebSocket lets the same server work over a direct `ip:port` connection and
//! through an HTTPS tunnel such as a Cloudflare quick tunnel.

use serde::{Deserialize, Serialize};

pub use dyno_sim::Stack;
pub use dyno_world::content::{Content, ItemId};

/// Survival or creative.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Survival,
    Creative,
}

/// A slot in the player's pack or in an open container.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotRef {
    Pack(u8),
    Chest(u8),
}

/// What an entity is.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EntityKind {
    /// a creature from the content, by creature ID
    Creature(u16),
    /// an item lying in the world
    Item(Stack),
    /// a friendly character from the content, by NPC ID
    Npc(u16),
    /// a fallen creature; `loot` while it still holds something
    Corpse { creature: u16, loot: bool },
}

/// A member of your party, as the frames show them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartyMember {
    pub name: String,
    pub level: u8,
    pub class: Option<u16>,
    pub health: f32,
    pub max: f32,
    pub online: bool,
    pub leader: bool,
}

/// A step in trading with another player.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TradeOp {
    /// ask a player (by id) to trade
    Ask(PlayerId),
    /// open the window with whoever asked you
    Agree,
    /// put a bag slot on the table (count 0: the whole stack)
    Offer {
        slot: u8,
        count: u8,
    },
    /// take a line back off the table
    Withdraw {
        line: u8,
    },
    /// coins on the table
    Money(u32),
    /// happy with both sides (anything changing clears it)
    Accept,
    Cancel,
}

/// The trade window: what each side has put on the table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TradeView {
    pub with: String,
    pub mine: Vec<Stack>,
    pub theirs: Vec<Stack>,
    pub my_money: u32,
    pub their_money: u32,
    pub i_accept: bool,
    pub they_accept: bool,
}

/// Who did something.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Actor {
    Player(PlayerId),
    Entity(u32),
}

/// A visible action, for animation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Act {
    /// a melee swing
    Swing,
    /// a bow shot or thrown weapon
    Shoot,
    /// starts casting `ability` for `time` seconds (0: instant)
    CastStart { ability: u16, time: f32 },
    /// stops casting (`done`: it went off)
    CastEnd { done: bool },
    /// an emote: an index into the client's animation list (`Anim::ALL`)
    Emote(u8),
    /// takes a blow
    Hurt,
    /// falls
    Died,
    /// gets up again
    Revived,
    /// a creature's special move (charge, bolt, roar)
    Special,
}

/// One quest in a player's log: counts per objective.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestProgress {
    pub quest: u16,
    pub counts: Vec<u8>,
}

/// Kinds of damage the client reports (the server owns everything else).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EnvDamage {
    Fall,
    Drown,
}

/// Network protocol version. Increase it on every incompatible change.
/// Clients and servers with different versions refuse to connect.
pub const PROTOCOL_VERSION: u32 = 16;

/// The ability ID used for the recall cast (back to your hearth or bed).
pub const RECALL: u16 = u16::MAX;
/// The ability ID used for a gathering cast (Mining, Herbalism, Woodcutting).
pub const GATHER: u16 = u16::MAX - 2;

/// Default TCP port of the game server.
pub const DEFAULT_PORT: u16 = 7400;

/// Largest message a client may send to the server, in bytes.
pub const MAX_CLIENT_MESSAGE: usize = 64 * 1024;
/// Largest message the server may send to a client, in bytes.
pub const MAX_SERVER_MESSAGE: usize = 32 * 1024 * 1024;
/// Longest player name, in characters.
pub const MAX_NAME: usize = 24;
/// Longest chat message, in characters.
pub const MAX_CHAT: usize = 256;
/// Largest fill, in cells.
pub const MAX_FILL: i64 = 32 * 32 * 32;

pub type PlayerId = u32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClientMsg {
    /// First message. The server answers with `Welcome` or `Reject`.
    Hello {
        protocol: u32,
        version: String,
        name: String,
    },
    /// The player's position and view direction.
    Pose(Pose),
    /// Request to change one cell.
    SetBlock {
        pos: [i32; 3],
        block: u8,
    },
    /// Chat text.
    Chat {
        text: String,
    },
    /// Fill the box between two corners (inclusive) with one block. Air clears it. Creative only.
    Fill {
        a: [i32; 3],
        b: [i32; 3],
        block: u8,
    },
    /// Second message. `token` identifies this install so names cannot be taken over.
    /// Observers (tools such as `dyno shot`) are invisible and cannot act.
    Session {
        token: u64,
        observer: bool,
        /// creating this character now (class and looks)
        create: Option<NewCharacter>,
    },
    /// Instead of a session: list my characters here, and the realm's status.
    ListCharacters {
        token: u64,
    },
    /// Instead of a session: delete one of my characters.
    DeleteCharacter {
        token: u64,
        name: String,
    },
    /// The hotbar slot in hand.
    Held {
        slot: u8,
    },
    /// Started breaking a block.
    DigStart {
        pos: [i32; 3],
    },
    /// Finished breaking a block.
    Dig {
        pos: [i32; 3],
    },
    /// Place the block item from a pack slot.
    Place {
        pos: [i32; 3],
        slot: u8,
    },
    /// Creative: put any item in a pack slot.
    CreativeSet {
        slot: u8,
        item: ItemId,
    },
    /// Move items between slots. `count` 0 moves the whole stack (and swaps if needed).
    Move {
        from: SlotRef,
        to: SlotRef,
        count: u8,
    },
    /// Throw items out of a slot. `count` 0 throws the whole stack.
    Throw {
        from: SlotRef,
        count: u8,
    },
    /// Craft a recipe `times` times.
    Craft {
        recipe: u16,
        times: u8,
    },
    /// Eat or use the item in hand.
    UseHeld,
    /// Hit an entity.
    Attack {
        entity: u32,
    },
    /// Right click on a block (chest, bed, table).
    Interact {
        pos: [i32; 3],
    },
    CloseContainer,
    /// Damage the client measured (falls, drowning).
    Hurt {
        kind: EnvDamage,
        amount: u8,
    },
    Respawn,
    /// Talk to an NPC (right click).
    Talk {
        entity: u32,
    },
    AcceptQuest {
        quest: u16,
    },
    /// Hand a finished quest in; `choice` picks a reward when there is a choice.
    CompleteQuest {
        quest: u16,
        choice: u8,
    },
    AbandonQuest {
        quest: u16,
    },
    /// Use a class ability, at a creature or at yourself.
    Cast {
        ability: u16,
        target: Option<u32>,
    },
    /// Pick a class (only while you have none).
    ChooseClass {
        class: u16,
    },
    /// Start the recall cast home.
    Recall,
    /// Eat the best food in your bags.
    EatBest,
    /// Wear the item in a bag slot.
    Equip {
        from: u8,
    },
    /// Take off what is worn in a paper-doll place (into the bags).
    Unequip {
        place: u8,
    },
    /// Start auto-attacking a creature, or stop (None).
    AutoAttack {
        target: Option<u32>,
    },
    /// Gather from a resource node (a cast; the node is not broken).
    Gather {
        pos: [i32; 3],
    },
    /// Sell a bag slot's stack to the NPC you are talking to.
    Sell {
        from: u8,
    },
    /// Open a corpse's loot (`all`: take everything at once).
    Loot {
        id: u32,
        all: bool,
    },
    /// Take one thing from an open corpse: an item slot, 254 the money, 255 all.
    LootTake {
        id: u32,
        slot: u8,
    },
    /// Buy from a merchant (an NPC entity).
    Buy {
        npc: u32,
        item: u16,
        count: u8,
    },
    /// Trading with another player.
    Trade(TradeOp),
    /// Play an emote for everyone nearby (an index into `Anim::ALL`).
    Emote(u8),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub struct Pose {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerInfo {
    pub id: PlayerId,
    pub name: String,
    pub pose: Pose,
    /// how they chose to look, and their class colour
    pub look: Appearance,
    pub color: [f32; 3],
}

/// How a character looks, chosen at creation (indices into the game's palettes).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub struct Appearance {
    pub skin: u8,
    pub eyes: u8,
    /// 0 bald, 1 short, 2 long, 3 hood, 4 bun
    pub hair_style: u8,
    pub hair_color: u8,
    pub beard: bool,
    /// 0 slight, 1 average, 2 burly
    pub build: u8,
    /// 0 cheerful, 1 stern, 2 calm
    pub mood: u8,
    /// 0 hidden, 1 round, 2 pointed
    pub ears: u8,
}

/// A character to create on joining: class (by key) and looks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewCharacter {
    pub class: String,
    pub look: Appearance,
}

/// One of your characters on a realm, for the character select screen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CharacterSummary {
    pub name: String,
    pub level: u8,
    pub class: String,
    pub color: [f32; 3],
    pub zone: String,
    pub look: Appearance,
}

/// A class as the creation screen shows it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClassInfo {
    pub key: String,
    pub name: String,
    pub color: [f32; 3],
    pub blurb: String,
    pub resource: String,
    pub armor: String,
    pub abilities: Vec<String>,
}

/// What a realm says about itself before you join.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealmInfo {
    pub name: String,
    pub motd: String,
    pub version: String,
    pub online: u16,
    pub classes: Vec<ClassInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMsg {
    Welcome {
        your_id: PlayerId,
        server_version: String,
        motd: String,
        size: [u32; 3],
        spawn: [f32; 3],
        players: Vec<PlayerInfo>,
        mode: Mode,
        content: Box<Content>,
    },
    /// The connection is refused. The server closes it after this message.
    Reject {
        reason: String,
    },
    /// The whole world, run-length encoded (see `dyno_world::World::encode`).
    World {
        data: Vec<u8>,
    },
    PlayerJoined(PlayerInfo),
    PlayerLeft {
        id: PlayerId,
    },
    /// Poses of all other players, sent 20 times per second.
    Poses(Vec<(PlayerId, Pose)>),
    /// One cell changed.
    Block {
        pos: [i32; 3],
        block: u8,
    },
    Chat {
        from: String,
        text: String,
    },
    System {
        text: String,
    },
    /// Time of day: 0.0 sunrise, 0.25 noon, 0.5 sunset, 0.75 midnight.
    /// `day_seconds` is the length of one full day.
    Time {
        time: f32,
        day_seconds: f32,
    },
    /// Move the player (from /tp and /spawn).
    Teleport {
        pos: [f32; 3],
    },
    /// A box of cells was filled with one block. `a` is the low corner, `b` the high corner.
    Fill {
        a: [i32; 3],
        b: [i32; 3],
        block: u8,
    },
    /// The player's whole pack (36 slots).
    Pack(Vec<Option<Stack>>),
    /// Health, hunger, mode and furnace fuel units.
    Status {
        health: f32,
        food: f32,
        mode: Mode,
        fuel: u16,
    },
    /// An open container's slots (`None` closes it).
    Container(Option<([i32; 3], Vec<Option<Stack>>)>),
    EntitySpawn {
        id: u32,
        kind: EntityKind,
        pos: [f32; 3],
        yaw: f32,
    },
    /// Positions and yaw of moving entities.
    EntityMoves(Vec<(u32, [f32; 3], f32)>),
    /// An entity left. `picked_by` is set when a player picked up an item.
    EntityGone {
        id: u32,
        picked_by: Option<PlayerId>,
    },
    EntityHurt {
        id: u32,
    },
    /// This player was hurt; `push` is a knockback velocity.
    Hurt {
        amount: f32,
        push: [f32; 3],
    },
    Died {
        message: String,
    },
    /// A milestone in the journal was reached.
    Milestone {
        title: String,
    },
    /// New content (after /reload).
    Content(Box<Content>),
    /// Dusk is coming (`dusk`), or a new day started (`day` is its number).
    DayEvent {
        dusk: bool,
        day: u32,
    },
    /// The block a player is breaking, for the crack effect (None when they stop).
    Digging {
        player: PlayerId,
        pos: Option<[i32; 3]>,
    },
    /// Your level, XP, money (copper) and most health.
    Character {
        class: Option<u16>,
        /// how you chose to look
        look: Appearance,
        level: u8,
        xp: u32,
        xp_next: u32,
        money: u32,
        max_health: f32,
    },
    /// You earned something: floating text and a sound.
    Gain {
        xp: u32,
        money: u32,
    },
    /// A player reached a level (shown to everyone).
    LevelUp {
        player: PlayerId,
        level: u8,
    },
    /// Level and health of a creature, for nameplates and the target frame.
    EntityInfo {
        id: u32,
        level: u8,
        health: f32,
        max: f32,
    },
    /// Your quest log and the quests you have finished.
    Quests {
        log: Vec<QuestProgress>,
        done: Vec<u16>,
    },
    /// Open a conversation with an NPC.
    Dialog {
        entity: u32,
        npc: u16,
    },
    /// Your rage or mana.
    Power {
        value: f32,
        max: f32,
    },
    /// A player started casting (`secs` long), or stopped (`ability` None).
    Casting {
        player: PlayerId,
        ability: Option<u16>,
        secs: f32,
    },
    /// Seconds left on your cooldowns, and on the global cooldown.
    Cooldowns {
        list: Vec<(u16, f32)>,
        global: f32,
    },
    /// Draw a spell: from a point to a point (a bolt), or a burst of `radius` at `to`.
    Effect {
        color: [f32; 3],
        from: [f32; 3],
        to: [f32; 3],
        radius: f32,
    },
    /// You have no class yet: choose one.
    ChooseClass,
    /// The answer to ListCharacters or DeleteCharacter.
    Characters {
        realm: RealmInfo,
        list: Vec<CharacterSummary>,
    },
    /// What you wear, by paper-doll place.
    Equipment(Vec<Option<Stack>>),
    /// Your auto-attack target (None when it stops).
    Attacking(Option<u32>),
    /// Your buffs and heals over time: ability, seconds left.
    Auras(Vec<(u16, f32)>),
    /// Combo points on a target.
    Combo {
        points: u8,
        target: Option<u32>,
    },
    /// The weather: 0 clear, 1 cloudy, 2 rain, 3 fog; and how strong (0..1).
    Weather {
        kind: u8,
        strength: f32,
    },
    /// A world event near you: title, what to do, progress 0..1 (empty title: it ended).
    WorldEvent {
        title: String,
        text: String,
        progress: f32,
    },
    /// A creature with its own name (a rare elite).
    EntityName {
        id: u32,
        name: String,
        elite: bool,
    },
    /// You entered a named place (`name` empty: the wilds).
    Zone {
        name: String,
        levels: [u8; 2],
        safe: bool,
    },
    /// Your professions: key, skill, cap.
    Skills(Vec<(String, u16, u16)>),
    /// Damage numbers: on a creature (`entity`) or on you (None).
    Damage {
        entity: Option<u32>,
        amount: f32,
        crit: bool,
    },
    /// A corpse's contents for you (nothing and no money: the window closes).
    Loot {
        id: u32,
        money: u32,
        items: Vec<Stack>,
    },
    /// Rested XP you have banked: kills give double XP until it runs out.
    Rested {
        pool: u32,
    },
    /// You moved to another copy (shard) of the world without leaving it:
    /// forget every creature, NPC and player you knew; these are the people
    /// here now, and the blocks that differ from where you were.
    Shard {
        shard: u32,
        players: Vec<PlayerInfo>,
        blocks: Vec<([i32; 3], u8)>,
    },
    /// Your party (empty: you are not in one).
    Party {
        members: Vec<PartyMember>,
    },
    /// Someone asks you to join their party ("/accept" or "/decline").
    Invite {
        from: String,
    },
    /// Someone or something does something visible: swings, shoots, casts, emotes.
    Act {
        actor: Actor,
        act: Act,
    },
    /// Someone asks to trade with you.
    TradeAsk {
        from: String,
    },
    /// The trade window (None: it closed).
    Trade(Option<TradeView>),
    /// What a player holds in their weapon hand (drawn on their figure).
    Wielding {
        player: PlayerId,
        item: Option<u16>,
    },
}

pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    postcard::to_stdvec(msg).expect("protocol messages always encode")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, postcard::Error> {
    postcard::from_bytes(bytes)
}

/// Explain a version mismatch to a person, naming the side that is older.
pub fn mismatch_reason(
    server_protocol: u32,
    server_version: &str,
    client_protocol: u32,
    client_version: &str,
) -> String {
    let older = if client_protocol < server_protocol {
        "your game"
    } else {
        "the server"
    };
    format!(
        "Version mismatch: the server runs dyno {server_version} (protocol {server_protocol}), \
         you have dyno {client_version} (protocol {client_protocol}). {older} is older. \
         Restart dyno to update, or ask the host to update the server."
    )
}

/// Normalise a player name: trim, limit length, keep printable characters.
pub fn clean_name(name: &str) -> String {
    let n: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME)
        .collect();
    if n.is_empty() {
        "player".to_string()
    } else {
        n
    }
}

/// Turn what a person typed into a WebSocket URL.
///
/// Accepts `host`, `host:port`, `1.2.3.4`, `[::1]:7400`, `ws://...`, `wss://...`
/// and `https://...` (for tunnels). A bare host uses `DEFAULT_PORT`.
pub fn server_url(input: &str) -> Result<String, String> {
    let s = input.trim();
    if s.is_empty() {
        return Err(
            "Enter a server address, for example 192.168.1.20 or https://name.trycloudflare.com"
                .into(),
        );
    }
    if s.chars().any(char::is_whitespace) {
        return Err("The address must not contain spaces.".into());
    }
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("https://") {
        return Ok(format!("wss://{}", &s[s.len() - rest.len()..]));
    }
    if let Some(rest) = lower.strip_prefix("http://") {
        return Ok(format!("ws://{}", &s[s.len() - rest.len()..]));
    }
    if lower.starts_with("ws://") || lower.starts_with("wss://") {
        return Ok(s.to_string());
    }
    // host part, without any path
    let (hostport, path) = match s.find('/') {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    };
    let has_port = if hostport.starts_with('[') {
        hostport.contains("]:")
    } else {
        hostport.matches(':').count() == 1
    };
    let hostport = if hostport.starts_with('[') || hostport.matches(':').count() <= 1 {
        hostport.to_string()
    } else {
        // bare IPv6 without brackets
        format!("[{hostport}]")
    };
    if has_port {
        Ok(format!("ws://{hostport}{path}"))
    } else {
        Ok(format!("ws://{hostport}:{DEFAULT_PORT}{path}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(server_url("1.2.3.4").unwrap(), "ws://1.2.3.4:7400");
        assert_eq!(server_url(" 1.2.3.4:9000 ").unwrap(), "ws://1.2.3.4:9000");
        assert_eq!(server_url("localhost").unwrap(), "ws://localhost:7400");
        assert_eq!(server_url("[::1]:7400").unwrap(), "ws://[::1]:7400");
        assert_eq!(server_url("[::1]").unwrap(), "ws://[::1]:7400");
        assert_eq!(server_url("::1").unwrap(), "ws://[::1]:7400");
        assert_eq!(
            server_url("https://abc.trycloudflare.com").unwrap(),
            "wss://abc.trycloudflare.com"
        );
        assert_eq!(
            server_url("HTTPS://Abc.trycloudflare.com/").unwrap(),
            "wss://Abc.trycloudflare.com/"
        );
        assert_eq!(server_url("wss://x.y").unwrap(), "wss://x.y");
        assert!(server_url("").is_err());
        assert!(server_url("a b").is_err());
    }

    #[test]
    fn roundtrip() {
        let m = ClientMsg::Hello {
            protocol: 1,
            version: "0.0.1".into(),
            name: "wren".into(),
        };
        assert_eq!(decode::<ClientMsg>(&encode(&m)).unwrap(), m);
    }

    #[test]
    fn names() {
        assert_eq!(clean_name("  "), "player");
        assert_eq!(clean_name("a\nb"), "ab");
        assert_eq!(clean_name(&"x".repeat(100)).len(), MAX_NAME);
    }
}
