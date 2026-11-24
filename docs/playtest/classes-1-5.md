# The six classes, levels 1 to 5

How each class plays in its first 30 minutes, before and after the class
pass of 23 to 24 November 2026 (branch `worktree-agent-a3b420e463eb200bb`,
protocol 24). It is judged against sections 6 and 7 of
[the quality bar](../design/quality-bar.md). The kits themselves are data in
`content/rules/hearthblock.json`; the table of abilities is in
[the RPG design](../design/rpg.md).

## What was measured, and how far to trust it

| Source | What it is | Trust it for |
|---|---|---|
| Swarm (`runs/classes-before.md`, `runs/classes-after.md`) | 12 bots, two per class, mixed brains, 30 minutes on a fresh world, one swarm at a time | Which buttons get pressed, kill times at levels 1 to 4, fights won |
| Duel model (`runs/classes-duels.md`) | 300 simulated duels per class and level with the server's combat formulas and the bots' rotation, against one and two same-level creatures | Kill time and damage taken at each level, the two-creature pull, levels 4 and 5 |
| Hand play (`runs/classes-play.txt`) | Each class through `dyno play` at levels 2 to 3 and 5, pressing each ability myself against summoned creatures. The file holds the level 2 to 3 session; the level 5 session's transcript was overwritten when the next one started, and its numbers survive only in the notes below | Feel: whether the buttons do what the tooltip says, and whether the resource matters |

Limits, stated plainly:
- **The bots do not reach level 5.** In 30 minutes they end at level 3 or 4
  (before and after), so the swarm says little about the level 4 to 5
  abilities. Those were checked in the duel model and by hand, with a new
  operator command (`/xp <amount>`) to put a test character at level 5.
- **Deaths cannot be compared between classes from these runs.** Two bots a
  class for 30 minutes is one bot-hour each. Most deaths are one or two bots
  walking into the bandit stockade again and again, or Shades at nightfall.
  The counts are given with their causes; they are not a class ranking.
- **The duel model is a model.** It agreed with the hand-played fights to
  within a second or two (while the hand fights had dead bleeds and the model
  did not, so the true agreement is unmeasured), and it stands the fighter
  still.
- **Hand play was shorter than asked.** I pressed the buttons for about 5
  minutes a class, not 10 to 15: the text interface takes a second a command
  and lets the body's own rotation finish a fight when I stop typing. The
  notes below say what I saw, not more.
- **Hand play ran with dead bleeds.** Every hand-played fight was before
  the bleed fix (see below): Rending Cut, Barbed Arrow and Withering Word
  cost their rage and mana in those fights and did no damage. Real kills are
  a little quicker than the ones in the notes.
- **Hand play ran on the numbers of the day.** The level 5 fights were
  played before the last retune (paladin seal and Judging Blow, warrior
  Heroic Blow and sword, rogue burst, priest shield). Where a note led to a
  change, it says so; the fights were not replayed afterwards.
- **The new sounds have not been heard by anyone.** They are synthesised by
  a script and checked only for level and length.

## Before: what was wrong

From the swarm on main (0f561ab), the duel model on the same rules, a
warrior played by hand on main and a read of the cast code. The creature
and area data are the same at 0f561ab and at the commit the "after" runs
were made on (checked field by field), though the world layout is not:

- **Bleeds did nothing at all.** Rending Cut, Barbed Arrow, Withering Word
  and Open Vein have no direct damage, so the server took them for
  abilities that need no target, cast them at nobody, charged for them and
  applied no bleed. On main the level 2 to 3 "second button" of the warrior,
  the hunter and the priest was a dead button. I did not notice this while
  playing (a bleed's ticks are easy to miss among auto-attacks); it came out
  of a review of the cast code after the kits were tuned. It is fixed, with a
  test, and the "before" columns of the duel model reproduce it.
- **The warrior's first press fails.** Rage starts at 0, Heroic Blow costs
  15, and the charge (Onrush) came at level 4. My first two presses as a new
  warrior answered "Not enough rage."
- **One button, and waiting.** Hunters had Quick Shot on a 6-second
  cooldown and nothing else at level 1; paladins had Judging Blow on an
  8-second cooldown. The model has both idle for more than half of their
  global cooldowns at levels 3 to 5.
- **No class fantasy before level 6.** No seal, no shield, no frost, no
  trap, no opener and no stealth existed; Ice Ring and Snare Shot came at
  level 6. A rogue short of energy was told "Not enough mana."
- **Low Blow was never worth pressing** (40 energy for 1 damage).
- **Kill times were far apart.** In the model at level 5, hunters took 72%
  longer than the mean and mages 45% less; in the swarm, warriors +36% and
  mages -27%.
- **Two creatures killed three classes.** In the model a level 5 warrior
  won 3% of two-bandit pulls, and a level 4 or 5 rogue or hunter almost
  none.
- **A press during the global cooldown was silently lost.**

The swarm on main (levels 1 to 4; `runs/classes-before.md`):

| Class | Bots | Fights (won) | Kill time, median s | same-level kill s | Damage taken / fight | Casts / fight | Presses per 10 s | Deaths (per hour) | Level after run | XP / hour |
|---|---|---|---|---|---|---|---|---|---|---|
| Warrior | 2 | 72 (94%) | 6.8 | 7.3 | 12.1 | 1.7 | 2.4 | 2 (2.0) | 4/4 | 3444 |
| Paladin | 2 | 120 (83%) | 4.7 | 6.1 | 9.5 | 1.3 | 2.0 | 20 (20.0) | 3/4 | 2650 |
| Hunter | 2 | 34 (94%) | 5.3 | 5.2 | 7.1 | 1.7 | 2.4 | 2 (2.0) | 3/3 | 2115 |
| Rogue | 2 | 43 (100%) | 4.8 | 3.6 | 7.9 | 2.2 | 4.4 | 0 (0.0) | 3/4 | 2266 |
| Priest | 2 | 71 (99%) | 4.7 | 4.6 | 3.3 | 2.4 | 4.9 | 1 (1.0) | 4/4 | 2970 |
| Mage | 2 | 66 (86%) | 3.6 | 4.2 | 2.5 | 2.0 | 4.7 | 9 (15.1) | 1/4 | 2411 |

| Class | Abilities (share of presses) | Deaths by cause |
|---|---|---|
| Warrior | Heroic Blow 54%, Rending Cut 28%, Battle Cry 14%, Onrush 4%, Recall 1% | Websnare (lv4) x1, Websnare (lv3) x1 |
| Paladin | Judging Blow 63%, Holy Mend 19%, Rebuke 17% | Rook Varn (lv3) x6, Ashcloak Bandit (lv3) x6, Ashcloak Cutthroat (lv3) x4, Shade (lv2) x2, Shade (lv3) x2 |
| Hunter | Quick Shot 57%, Barbed Arrow 40%, Recall 3% | Websnare (lv1) x1, Mirepincher (lv3) x1 |
| Rogue | Vicious Strike 79%, Finishing Cut 20%, Recall 1% | none |
| Priest | Smite 76%, Withering Word 21%, Recall 2%, Mend 1% | Shade (lv3) x1 |
| Mage | Fireball 69%, Fire Blast 30%, Scholar's Focus 1% | Ashcloak Bandit (lv3) x6, Shade (lv2) x1, Ashcloak Cutthroat (lv3) x1, Rook Varn (lv3) x1 |

Kill time spread vs mean (5.0 s): Warrior +36%, Paladin -6%, Hunter +6%, Rogue -4%, Priest -6%, Mage -27%

| Class | L1 kill s (n) | L2 kill s (n) | L3 kill s (n) | L4 kill s (n) | L5 kill s (n) |
|---|---|---|---|---|---|
| Warrior | 3.9 (10) | 5.8 (16) | 7.4 (31) | 9.4 (11) | - |
| Paladin | 3.5 (14) | 4.6 (31) | 6.2 (54) | 6.7 (1) | - |
| Hunter | 4.8 (7) | 4.9 (13) | 6.5 (12) | - | - |
| Rogue | 3.2 (8) | 3.6 (15) | 6.8 (14) | 8.8 (6) | - |
| Priest | 3.2 (23) | 4.7 (17) | 4.9 (25) | 4.7 (5) | - |
| Mage | 3.5 (11) | 4.1 (11) | 3.6 (32) | 6.1 (3) | - |

## What changed

### The kits (data)

Every class now has two or three buttons at level 1, a new ability at level
2 or 3 and another at 4 or 5. New abilities are marked (new); the rest were
moved down from later levels or retuned.

**Warrior** (rage)

| Level | Ability | Cost | Cooldown | Cast | Range | What it does |
|---|---|---|---|---|---|---|
| 1 | Heroic Blow | 15 rage | - | instant | melee | A mighty blow: weapon damage plus more. |
| 1 | Onrush | - | 12 s | instant | 25 yd | Rush to your target, stun it for 1.5 seconds and gain 20 rage. |
| 2 | Second Wind (new) | 15 rage | 20 s | instant | self | Turn your rage into resolve: heal your wounds. |
| 3 | Rending Cut | 10 rage | - | instant | melee | Open a wound that bleeds for 12 seconds. |
| 4 | Ground Slam | 20 rage | 8 s | instant | 5 yd around you | Slam the earth: every foe within 5 yards is hurt and slowed by 40% for 4 seconds. |
| 6 | Battle Cry | 10 rage | - | instant | self | Your cry steels you: 10% more damage for 2 minutes. |

**Paladin** (mana)

| Level | Ability | Cost | Cooldown | Cast | Range | What it does |
|---|---|---|---|---|---|---|
| 1 | Judging Blow | 7 mana | 4 s | instant | melee | Strike with holy might: weapon damage plus more. Shades take 50% more. |
| 1 | Holy Mend | 18 mana | - | 2 s | self | Call on the light to heal your wounds. |
| 1 | Seal of Dawn (new) | 10 mana | - | instant | self | Seal your weapon with dawnlight for 60 seconds: every swing burns for extra holy damage. |
| 2 | Valour | 20 mana | - | instant | self | Fight with valour: 10% more damage for 5 minutes. |
| 3 | Rebuke | 10 mana | 30 s | instant | melee | A stern blow that stuns your target for 3 seconds. |
| 5 | Devout Ward | 15 mana | 60 s | instant | self | Take 30% less damage for 10 seconds. |

**Hunter** (mana)

| Level | Ability | Cost | Cooldown | Cast | Range | What it does |
|---|---|---|---|---|---|---|
| 1 | Quick Shot | 6 mana | 3 s | instant | 30 yd | A fast, aimed shot. |
| 1 | Barbed Arrow | 8 mana | - | instant | 30 yd | A barbed arrow that keeps cutting for 12 seconds. |
| 2 | Marked Shot (new) | 10 mana | 10 s | 1.5 s | 30 yd | Take careful aim for a heavy shot. 50% more against a foe that is not yet fighting you. |
| 3 | Hamstring (new) | 8 mana | 8 s | instant | melee | A low cut with your hatchet: weapon damage, and the target is slowed by 50% for 6 seconds. |
| 4 | Snare Trap | 10 mana | 15 s | instant | 30 yd | Loose a snare at your target's feet: it is held in place for 5 seconds. |

**Rogue** (energy)

| Level | Ability | Cost | Cooldown | Cast | Range | What it does |
|---|---|---|---|---|---|---|
| 1 | Vicious Strike | 45 energy | - | instant | melee | Weapon damage plus a little more. Adds a combo point. |
| 1 | Finishing Cut | 25 energy | - | instant | melee | A finisher: damage for each combo point spent. |
| 2 | Ambush (new) | 45 energy | 10 s | instant | melee | A strike from the blind side: 40% more damage against a foe that is not yet fighting you. Adds 2 combo points. |
| 3 | Low Blow | 25 energy | 12 s | instant | melee | Stun your target for 4 seconds. Adds a combo point. |
| 4 | Slip Away | - | 60 s | instant | self | Twist away from every blow: take 60% less damage for 8 seconds. |
| 5 | Sneak (new) | 20 energy | 6 s | instant | self | Slip into the shadows for 30 seconds: creatures notice you from much less far. Ends when you strike or are struck. |
| 6 | Flurry | 25 energy | - | instant | self | A finisher: attack 30% faster for 6 seconds, plus 3 per combo point. |

**Priest** (mana)

| Level | Ability | Cost | Cooldown | Cast | Range | What it does |
|---|---|---|---|---|---|---|
| 1 | Smite | 9 mana | - | 1.5 s | 25 yd | Holy fire burns your target. Shades take double. |
| 1 | Mend | 15 mana | - | 2 s | self | Close your wounds. |
| 2 | Withering Word | 12 mana | - | instant | 25 yd | A word of shadow eats at your target for 12 seconds. |
| 3 | Ward of Light (new) | 14 mana | 15 s | instant | self | A shield of light soaks up damage for 15 seconds, then breaks. |
| 5 | Renewal | 16 mana | - | instant | self | Heal yourself over 15 seconds. |

**Mage** (mana)

| Level | Ability | Cost | Cooldown | Cast | Range | What it does |
|---|---|---|---|---|---|---|
| 1 | Fireball | 14 mana | - | 2 s | 25 yd | Hurl a ball of fire. |
| 1 | Frostbolt (new) | 10 mana | - | 1.5 s | 25 yd | A bolt of ice that slows your target by 50% for 6 seconds. |
| 2 | Fire Blast | 12 mana | 8 s | instant | 20 yd | An instant burst of flame. |
| 4 | Ice Ring | 18 mana | 25 s | instant | 6 yd around you | Frost holds every foe within 6 yards in place for 5 seconds. |
| 5 | Scholar's Focus | 15 mana | - | instant | self | +5 Intellect for 30 minutes. |

Other data changes:
- Rogue stamina growth 10 to 12 a level; warrior 14 to 16; mage base stamina 5 to 6.
- Warriors and paladins can wear plate and hunters mail (no class could wear plate, and the hunter's mail set did not fit hunters; found by the flow lane).
- Starter gear (per-class items): Worn Shortsword 3-5 to 4-6; Battered
  Buckler armour 12 to 16; the warrior's and paladin's tabard and pauldrons,
  and the rogue's and hunter's jerkin and hood, carry real armour (12 and 6;
  10 and 5; 7 and 3; 6 and 3) instead of 3 and 1.
- Each ability has its own colour within its class (the icon tile and the
  spell effect), its own icon (`icons/abilities/<key>.png`) and a sound
  (`sounds/abilities/<name>_<n>.wav`).

### General capabilities added to the engine (each one is data-driven)

| Capability | Data | Used by |
|---|---|---|
| Bleeds (damage over time with no direct damage) are cast at their target; before, they were cast at nobody and did nothing | none (a fix) | Rending Cut, Barbed Arrow, Withering Word, Open Vein |
| Opener: more damage against a creature that is not yet fighting you (or was first hit within 1.5 s, so an auto-attack landing first does not waste it) | `opener: 1.4` | Ambush, Marked Shot |
| Slow | `slow: [percent, seconds]` | Frostbolt, Hamstring, Ground Slam |
| Absorb shield: soaks up damage, then breaks ("Your shield absorbs 8.") | buff `absorb` | Ward of Light |
| Weapon seal: extra damage on each auto-attack | buff `strike` | Seal of Dawn |
| Stealth: creatures notice you from less far; ends when you strike or are struck; not usable in combat | buff `stealth` | Sneak |
| Buff strength that grows with level | `per_level` | Ward of Light, Seal of Dawn |
| An ability's own icon (in the pack before play) and sound (streamed when first needed) | `image`, `sound` | all 43 abilities |
| Ability queue: a press up to 0.4 s before the global cooldown or a cast ends goes off the moment it can | none (server) | everyone |
| Heal numbers: a green "+N" and a combat-log line when you are healed | none (`Healed` message) | every heal |
| A flash for buffs that had no effect to show | none | Battle Cry, Valour, Sneak and the like |
| Level-up lines: "You have learned Ambush: ..." and full mana | none | every level-up |
| "Not enough energy." for rogues | none | rogues |
| `/xp <amount>` for operators | none | testing a class at a level |

The wire format changed (`Effect` carries its ability; `Healed` is new; the
ability and buff data have new fields), so the protocol is 24.

### The bots

The rotation in `dyno-bot` learned what a player would do with the new
kits, keyed on what an ability does, never on a class or ability name: an
absorb shield as soon as something is on you, a defensive cooldown earlier
when two creatures are on you, stealth before walking up to a fresh foe, an
opener first, a stun on what is in reach, a root or slow on what is still
coming, a ground root when surrounded or hurt. `dyno play`'s `cast` now
starts auto-attack on its target, as the client does.

## After

The swarm on this branch (levels 1 to 4; `runs/classes-after.md`). It was the fourth run; the first (before the bleed fix and the last retunes) had kill-time medians from -22% to +24% of the mean, the third from -36% to +26%, which led to the last retune of smite, Quick Shot and the rogue's strikes. One run each, two bots a class: the spread between runs is about as large as the spread between classes. Applied after this run and so not in it: Slip Away 60% on a 60-second cooldown, plate for warriors and paladins and mail for hunters, one more point of mage stamina, and main's Grazer damage (3 to 2.5):

| Class | Bots | Fights (won) | Kill time, median s | same-level kill s | Damage taken / fight | Casts / fight | Presses per 10 s | Deaths (per hour) | Level after run | XP / hour |
|---|---|---|---|---|---|---|---|---|---|---|
| Warrior | 2 | 64 (95%) | 5.0 | 4.5 | 7.9 | 2.2 | 3.6 | 0 (0.0) | 4/4 | 2826 |
| Paladin | 2 | 100 (98%) | 5.9 | 6.3 | 10.5 | 2.0 | 3.0 | 1 (1.0) | 3/4 | 2659 |
| Hunter | 2 | 43 (100%) | 4.9 | 4.9 | 3.3 | 2.5 | 3.6 | 0 (0.0) | 3/4 | 2573 |
| Rogue | 2 | 48 (98%) | 4.4 | 4.6 | 4.7 | 1.8 | 3.5 | 1 (1.0) | 3/3 | 2606 |
| Priest | 2 | 68 (100%) | 4.7 | 4.7 | 2.1 | 2.7 | 5.4 | 0 (0.0) | 4/4 | 3379 |
| Mage | 2 | 44 (95%) | 5.4 | 5.7 | 5.1 | 2.5 | 4.3 | 2 (2.8) | 3/3 | 2570 |

| Class | Abilities (share of presses) | Deaths by cause |
|---|---|---|
| Warrior | Heroic Blow 47%, Onrush 24%, Rending Cut 22%, Ground Slam 3%, Second Wind 3%, Recall 2% | none |
| Paladin | Judging Blow 63%, Seal of Dawn 17%, Rebuke 10%, Holy Mend 6%, Valour 4% | Shade (lv2) x1 |
| Hunter | Quick Shot 57%, Barbed Arrow 33%, Marked Shot 4%, Recall 4%, Hamstring 2% | none |
| Rogue | Vicious Strike 43%, Finishing Cut 28%, Ambush 13%, Low Blow 13%, Recall 2% | Shade (lv2) x1 |
| Priest | Smite 64%, Withering Word 29%, Ward of Light 4%, Recall 3% | none |
| Mage | Fireball 58%, Frostbolt 22%, Fire Blast 19%, Recall 1% | Shade (lv3) x1, Shade (lv2) x1 |

Kill time spread vs mean (5.1 s): Warrior -2%, Paladin +17%, Hunter -3%, Rogue -12%, Priest -7%, Mage +8%

| Class | L1 kill s (n) | L2 kill s (n) | L3 kill s (n) | L4 kill s (n) | L5 kill s (n) |
|---|---|---|---|---|---|
| Warrior | 3.3 (5) | 3.2 (23) | 5.7 (24) | 5.9 (9) | - |
| Paladin | 5.5 (14) | 4.2 (40) | 8.0 (33) | 7.8 (11) | - |
| Hunter | 4.8 (8) | 4.7 (18) | 6.8 (17) | - | - |
| Rogue | 3.7 (5) | 3.0 (17) | 5.1 (25) | - | - |
| Priest | 3.3 (7) | 4.7 (21) | 4.7 (35) | 4.7 (5) | - |
| Mage | 5.8 (11) | 5.1 (19) | 3.9 (12) | - | - |

### One creature of your level (duel model, 300 fights each)

| Class | Level 3: kill s / health lost / idle cooldowns | Level 5: kill s / health lost / idle cooldowns |
|---|---|---|
| Warrior | 8.3 / 42% / 62% → 4.7 / 15% / 0% | 7.8 / 43% / 21% → 7.3 / 36% / 15% |
| Paladin | 8.8 / 24% / 62% → 5.3 / 8% / 37% | 9.4 / 34% / 66% → 6.4 / 20% / 39% |
| Hunter | 11.5 / 25% / 61% → 6.2 / 0% / 23% | 13.4 / 48% / 65% → 7.6 / 0% / 18% |
| Rogue | 5.1 / 10% / 38% → 5.5 / 10% / 27% | 5.1 / 13% / 38% → 6.9 / 17% / 35% |
| Priest | 6.4 / 12% / 0% → 6.3 / 12% / 0% | 6.7 / 19% / 0% → 6.3 / 16% / 0% |
| Mage | 4.2 / 0% / 0% → 6.6 / 0% / 0% | 4.2 / 0% / 0% → 6.7 / 0% / 0% |
| Spread of kill time about the mean | -43% to +56% → -19% to +14% | -45% to +72% → -8% to +11% |

Level 3 is against a Thornboar, level 5 against an Ashcloak Bandit. "Idle
cooldowns" is the share of global cooldowns with nothing worth pressing. At
levels 1 and 2 a Grazer dies in 2 to 5 seconds whatever the class; the
spread there is wide in percent (warrior -28%, mage +36% at level 1) and
about a second and a half in fact.

### Two creatures of your level at once (duel model)

| Class | Level 3: won / health left | Level 4 | Level 5 |
|---|---|---|---|
| Warrior | 31% / 1% → 100% / 57% | 37% / 2% → 100% / 59% | 3% → 100% / 44% |
| Paladin | 100% / 42% → 100% / 88% | 93% / 18% → 100% / 60% | 94% / 42% → 100% / 59% |
| Hunter | 14% / 1% → 100% / 72% | 0% → 100% / 51% | 0% → 100% / 36% |
| Rogue | 100% / 16% → 100% / 14% | 6% → 98% / 17% | 0% → 91% / 5% |
| Priest | 100% / 38% → 100% / 75% | 100% / 55% → 100% / 74% | 100% / 77% → 100% / 86% |
| Mage | 100% / 82% → 100% / 63% | 100% / 100% → 100% / 77% | 100% / 73% → 100% / 49% |

Two creatures are now dangerous and winnable for everyone. The rogue is
the one that must play well: it wins with a sliver of health, and loses
about one pull in ten at level 5.

## Per class: before, after and how it played

The play notes are first person and from the text interface: I typed each
`cast`, and read health, resource and the creature's health after each one.
Level 5 fights were against a summoned level 4 or 5 Ashcloak Bandit or
Mirepincher on the East Road; level 2 to 3 fights against summoned
Thornboars in Hearthmeadow.

### Warrior

| | Before | After |
|---|---|---|
| Buttons by level 5 | Heroic Blow, Battle Cry (2), Rending Cut (3), Onrush (4) | Heroic Blow, Onrush (1), Second Wind (2), Rending Cut (3), Ground Slam (4) |
| First press of a new character | "Not enough rage." | Onrush: a charge, a stun and 20 rage; Heroic Blow lands on the next cooldown |
| Most-pressed button's share (swarm) | Heroic Blow 54% | Heroic Blow 48% (5 abilities pressed) |

Before, at level 1, I pressed Heroic Blow twice at a Grazer and got "Not
enough rage" twice; the fight was three auto-attacks and one blow. After, at
level 1: Onrush put me on the Grazer with 24 rage, and three Heroic Blows in
a row all landed (rage 24, 30, 21, 8). That is the class in ten seconds.

At level 5 against a level 5 bandit (106 health): Onrush, Rending Cut,
Heroic Blow, Ground Slam, dead in about 8 seconds. Rage rose through the
fight (31 to 48) and I never waited for it; if anything I ended with 39
unspent. I was at 34 of 85 health afterwards. Second Wind took me back to 71
for 15 rage. It is the button I would miss most: before it, a warrior ate or
sat after every second fight (two level 2 to 3 boars, with ten seconds'
rest between, took me from 65 to 25).
Ground Slam's slow did nothing I could see in a one-creature fight.

### Paladin

| | Before | After |
|---|---|---|
| Buttons by level 5 | Judging Blow (8 s cooldown), Holy Mend, Rebuke (3), Valour (4) | Judging Blow (4 s), Holy Mend, Seal of Dawn (1), Valour (2), Rebuke (3), Devout Ward (5) |
| Idle cooldowns at level 5 (model) | 66% | 39% |
| Most-pressed button's share (swarm) | Judging Blow 63% | Judging Blow 63% (5 abilities pressed) |

At level 1 on my first retune (Judging Blow on 6 seconds; it was 8 on
main) I pressed it, then got "That is not ready yet" on the next two tries;
the rest was the maul swinging by itself. So it went to 4 seconds and the
seal came down to level 1: seal the weapon, judge, and judge again four
seconds later. It is
still the slowest hands of the six: at level 5 two of every five cooldowns
have nothing to press, which is the paladin's nature with a 3.2-second maul,
but it no longer feels broken.

At level 3 with the seal and Valour up, one Judging Blow and the swing
beside it took a level 1 boar from 48 to 0. At level 5 a level 4 bandit (92
health) fell in about 6 seconds to the seal, two blows and Rebuke; I lost 25
of 80 health and Holy Mend put it back for 18 mana. Mana never mattered in
one fight (84 to 59, most of it Devout Ward and the seal). The bandit fled
at 2 health while I had only melee buttons, which is the one moment the
class felt helpless.

### Hunter

| | Before | After |
|---|---|---|
| Buttons by level 5 | Quick Shot (6 s cooldown), Barbed Arrow (2) | Quick Shot (3 s), Barbed Arrow (1), Marked Shot (2), Hamstring (3), Snare Trap (4) |
| Idle cooldowns at level 5 (model) | 65% | 18% |
| Kill time against the mean, level 5 (model) | +72% | +11% |
| Most-pressed button's share (swarm) | Quick Shot 57% | Quick Shot 59% (4 abilities pressed) |

At level 5 from 22 blocks: Marked Shot took a level 5 Mirepincher from 117
to 73 before it knew I was there, which is the best single moment in the six
kits. Barbed Arrow and Quick Shot as it walked in, Snare Trap when it
reached 5 blocks: it stood there at 4 blocks while I shot it dead. No damage
taken, and half my mana gone (68 to 33), so two pulls in a row need a
breath. Started in melee instead (a boar summoned on top of me at level 3) I
lost 20 of 60 health and used Hamstring as a plain hit; without stepping
back its slow buys nothing, and the text interface cannot kite. There is no
pet, and the "trap" is a shot.

### Rogue

| | Before | After |
|---|---|---|
| Buttons by level 5 | Vicious Strike, Finishing Cut, Low Blow (3; 1 damage for 40 energy) | Vicious Strike, Finishing Cut, Ambush (2), Low Blow (3; 25 energy, 4 s stun), Slip Away (4), Sneak (5) |
| Two level 5 bandits (model) | 0% won | 91% won |
| Most-pressed button's share (swarm) | Vicious Strike 79% | Vicious Strike 44% (4 abilities pressed) |

At level 5 on a level 5 Mirepincher: Ambush (117 to 85, two combo points),
Vicious Strike (to 49), Low Blow (to 32, a fourth point, stunned), Finishing
Cut: dead in under 5 seconds with 16 energy left. This is the clearest
rotation of the six, and the energy bar is the pacing: after that burst I
could not have pressed anything for three seconds. That first hand run was
about 45% quicker than the hunter's and warrior's kills, so Ambush's bonus
(60% to 40%), Vicious Strike and Finishing Cut were turned down afterwards;
the model now has the rogue at the mean at level 5. I did not replay the
fight after that change. The last swarm showed rogues still the quickest at
levels 1 and 2, so Vicious Strike and Ambush now use 85% of the weapon
swing, and Slip Away went from 50% for 90 seconds' cooldown to 60% for 60
to keep the two-creature pull winnable.

Sneak cast, cost its energy and showed its effect, and it refuses in
combat. I did not test it properly: summoned bandits attack at once, so I
never walked up to one unseen. The aggro change is covered by the code path
only.

### Priest

| | Before | After |
|---|---|---|
| Buttons by level 5 | Smite, Mend, Withering Word (2) | Smite, Mend, Withering Word (2), Ward of Light (3), Renewal (5) |
| Most-pressed button's share (swarm) | Smite 76% | Smite 66% (3 abilities pressed) |

With a level 4 bandit on me at level 5: Ward of Light soaked five blows
("Your shield absorbs 8." five times, 39 in all) while Withering Word and
two Smites went out; I finished at 41 of 60 with Renewal ticking, and the
bandit died running. Mana 124 to 68: the shield, the shadow and the renewal
are a real budget, and three such fights empty the bar. The shield was too
good at 39 on a 12-second cooldown, so it is now 33 on 15. The five absorb
lines are a lot of text; a shield bar would say it better. Smite is still
most of what a priest presses.

### Mage

| | Before | After |
|---|---|---|
| Buttons by level 5 | Fireball, Scholar's Focus (2), Fire Blast (2) | Fireball, Frostbolt (1), Fire Blast (2), Ice Ring (4), Scholar's Focus (5) |
| Kill time against the mean, level 5 (model) | -45% | -2% |
| Most-pressed button's share (swarm) | Fireball 69% | Fireball 59% (3 abilities pressed) |

With a level 5 bandit summoned on top of me: Ice Ring, eight steps back
(it stayed where it was), Frostbolt, Fireball, Fire Blast: dead before it
reached me again, 106 health in three spells. That is frost and fire
control as asked. I took 19 damage before the ring went off. Mana 156 to
112, so about three kills a bar. Fireball still hits very hard (43 of 106);
the mage's kill time is level with the others now only because Frostbolt is
weaker and opens the fight.

## Against the quality bar

### Section 6: combat and classes

| Line | State | Evidence and what is missing |
|---|---|---|
| The fantasy reads within a minute | Mostly met | Warrior charge and rage, paladin seal and heal, hunter shots and bleed, rogue combo points, priest heal, mage frost and fire are all on the bar at level 1. Traps (Snare Trap is a snare shot, level 4), the priest's shield (level 3) and stealth (level 5) come later, and stealth is only a smaller aggro range: nothing hides the character on screen. There is no hunter pet. |
| Two to three abilities by level 5, new at 2-3 and 4-5 | Met | 5 or 6 a class by level 5; held by the test `every_class_has_a_kit_by_level_five`. |
| Feel: responsive cooldown, hits that land | Partly met | The ability queue and heal numbers are new. Swing animation, impact effect, floating damage and the yellow crit number were there already. Not checked in a window: I worked headless, so I have not seen or heard any of it. |
| Resources matter | Met by hand play | Rage climbs through a fight and Second Wind spends it on health; a rogue ends an opener at 16 energy; a hunter spends half its mana on one level 5 kill; a priest 45% on a fight with shield, shadow and renewal. Paladin and mage mana barely move in one fight. |
| A single same-level creature is safe | Met in the model | 100% wins for every class and level; the warrior still loses 36% of its health to a level 5 bandit. |
| Two are dangerous but winnable | Met in the model | See the table; rogue 91% at level 5. |
| Deaths under 1.5 an hour | Met in one run, not shown | 34 deaths in the swarm before (6.1 an hour: Ashcloak Bandit 12, Rook Varn 7, Shade 6, Ashcloak Cutthroat 5) and 4 after (0.7 an hour, all Shades). Do not read this as a sixfold gain: the two runs between them had 23 and 14 deaths (4.0 and 2.5 an hour), and the difference is mostly whether one or two bots walk into the bandit stockade and keep going back. Six bot-hours is too little to show a rate of 1.5. |
| Classes within 20% in kill time and deaths | Kill time: met at levels 3 to 5 in the model, and in the swarm (levels 1 to 4) the class medians run from -12% to +17% of the mean (before: -27% to +36%). Deaths: cannot be told from these runs. |
| Party tagging, loot and XP | Not touched | |

### Section 7: abilities (presentation)

| Line | State | Evidence and what is missing |
|---|---|---|
| Icons read at 40 px and are distinct within a class | Met for distinctness, by eye for legibility | 43 drawn icons, one per ability, none shared (test). I judged them on a contact sheet at 3x, not in the client. |
| Tooltips from data, worded consistently | Met | The tooltip was already built from the data (cost, range or radius, cast, cooldown, text). Every text was rewritten to one pattern, and a test checks that each "N seconds" in a text is a number in that ability's data. It shows "5 yd range" for melee rather than "Melee range": that line is the UI lane's. |
| A cast effect, an impact effect and a sound for every ability | Partly met | Every ability now sends an effect (buff-only ones did not) and names a sound; the effects are still the two generic ones (a bolt, a burst) in the ability's colour. The sounds are unheard. |
| Cooldown sweep, tints, keybind numbers | UI lane | The sweep, the out-of-resource tint and the key numbers exist. There is no out-of-range tint. |

## What is next

- **Listen to the 22 sounds and look at the icons in the client.** Both were
  made blind.
- **A real swarm at levels 4 and 5**: let `dyno bots` start characters at a
  level, so the late abilities and deaths per class can be measured rather
  than modelled, and run six bot-hours a class when the machine is free.
- **Melee still pays in health.** A warrior loses a third of its health to
  one level 5 bandit; ranged classes lose almost none. Creature damage is
  the flow lane's; the class-side levers (armour, Second Wind) are used.
- **Stealth you can see, traps you can place, and a hunter pet** need more
  than data: a hidden state other clients respect, a ground object that
  springs, a companion with its own AI.
- **Spell effects per ability** (a frost bolt that looks like frost): the
  particle system is data already (`life.json`); abilities could name an
  effect there as they now name a sound.
- **For the UI lane:** an out-of-range tint, "Melee range" in the tooltip, a
  place for the "You have learned ..." line on the level-up banner, new
  abilities placed on the bar, and a shield bar for absorbs.
