# Class duels: the model, before and after

300 simulated duels per class and level, with the server's combat formulas (ability damage, auto-attacks, armour, resources, the global cooldown) and the bots' rotation. The fighter stands still; a ranged class starts at its range less 4 blocks, a warrior at charge range, the others in reach. Long buffs are already running.

Columns: fights won (%), seconds to kill, damage taken as a share of full health (over 100% means it healed), health left, resource left, the share of global cooldowns with nothing to press, and each ability's share of presses. The last lines give each class's kill time against the mean (warrior, paladin, hunter, rogue, priest, mage) and the damage taken.

## Before (main 0f561ab), one creature

```
class    lv mob           win%   ttk taken% hp left pow left idle%  casts
warrior   1 grazer 1       100   4.2     14      86        1    98  heroic_blow:100
warrior   2 grazer 2       100   4.1     15      85        4    96  heroic_blow:100
warrior   3 thornboar 3    100   7.9     35      65        9    61  rending_cut:50 heroic_blow:50
warrior   4 mirepinch 4    100   7.1     32      68        5    20  heroic_blow:49 onrush:25 rending_cut:25
warrior   5 ashcloak 5     100   6.3     34      66        8    13  heroic_blow:49 onrush:25 rending_cut:25

paladin   1 grazer 1       100   1.4      4      96       81    22  judging_blow:100
paladin   2 grazer 2       100   3.0     10      90       81    48  judging_blow:100
paladin   3 thornboar 3    100   8.8     24      76       50    62  judging_blow:64 rebuke:35
paladin   4 mirepinch 4    100   9.4     29      71       54    66  judging_blow:65 rebuke:34
paladin   5 ashcloak 5     100   9.4     34      66       59    66  judging_blow:65 rebuke:34

hunter    1 grazer 1       100   5.3      0     100       80    69  quick_shot:100
hunter    2 grazer 2       100   5.2      0     100       60    38  barbed_arrow:50 quick_shot:50
hunter    3 thornboar 3    100   7.9      6      94       51    57  quick_shot:66 barbed_arrow:33
hunter    4 mirepinch 4    100   9.7      5      95       51    54  quick_shot:66 barbed_arrow:33
hunter    5 ashcloak 5     100   8.8     21      79       57    52  quick_shot:66 barbed_arrow:33

rogue     1 grazer 1       100   1.6      5      95       35     0  vicious_strike:94 finishing_cut:5
rogue     2 grazer 2       100   2.0      6      94       15     0  vicious_strike:68 finishing_cut:31
rogue     3 thornboar 3    100   5.1     10      90        1    38  vicious_strike:50 low_blow:25 finishing_cut:24
rogue     4 mirepinch 4    100   5.3     12      88        2    38  vicious_strike:50 low_blow:25 finishing_cut:25
rogue     5 ashcloak 5     100   5.1     13      87        0    38  vicious_strike:50 low_blow:25 finishing_cut:25

priest    1 grazer 1       100   3.9      0     100       72     0  smite:100
priest    2 grazer 2       100   4.7      0     100       64     0  smite:66 withering_word:33
priest    3 thornboar 3    100   6.3     11      89       59     0  smite:74 withering_word:25
priest    4 mirepinch 4    100   6.4      0     100       64     0  smite:75 withering_word:24
priest    5 ashcloak 5     100   6.3     16      84       67     0  smite:75 withering_word:25

mage      1 grazer 1       100   4.1      0     100       76     0  fireball:100
mage      2 grazer 2       100   4.1      0     100       77     0  fireball:100
mage      3 thornboar 3    100   4.2      0     100       72     0  fireball:69 fire_blast:30
mage      4 mirepinch 4    100   4.3      0     100       73     0  fireball:67 fire_blast:32
mage      5 ashcloak 5     100   4.2      0     100       76     0  fireball:69 fire_blast:30

lv1: mean ttk 3.4  spread +23% -58% +55% -54% +14% +20%   taken 14 4 0 5 0 0
lv2: mean ttk 3.9  spread +6% -21% +35% -49% +23% +7%   taken 15 10 0 6 0 0
lv3: mean ttk 6.7  spread +18% +31% +18% -24% -6% -37%   taken 35 24 6 10 11 0
lv4: mean ttk 7.0  spread +0% +34% +38% -24% -9% -39%   taken 32 29 5 12 0 0
lv5: mean ttk 6.7  spread -6% +41% +32% -24% -5% -37%   taken 34 34 21 13 16 0
```

## Before, two creatures at once

```
class    lv mob           win%   ttk taken% hp left pow left idle%  casts
warrior   1 grazer 1       100   8.8     39      61        4    76  heroic_blow:100
warrior   2 grazer 2       100   9.8     47      53       12    69  heroic_blow:100
warrior   3 thornboar 3     55  13.6    101       3       10    45  heroic_blow:60 rending_cut:39
warrior   4 mirepinch 4    100  13.7     83      17        5    27  heroic_blow:57 rending_cut:28 onrush:14
warrior   5 ashcloak 5      29  11.6     97       4        9    13  heroic_blow:57 rending_cut:28 onrush:14

paladin   1 grazer 1       100   8.4     30      70       65    78  judging_blow:100
paladin   2 grazer 2       100   9.6     41      60       64    71  judging_blow:99 holy_mend:0
paladin   3 thornboar 3    100  20.6    131      42        7    60  judging_blow:59 rebuke:20 holy_mend:20
paladin   4 mirepinch 4     93  34.2    221      18       13    72  judging_blow:40 holy_mend:39 rebuke:20
paladin   5 ashcloak 5      94  26.2    211      42        5    57  judging_blow:50 holy_mend:33 rebuke:16

hunter    1 grazer 1       100  12.2      0     100       51    73  quick_shot:100
hunter    2 grazer 2       100  12.0      0     100       21    48  barbed_arrow:50 quick_shot:50
hunter    3 thornboar 3    100  19.2     73      27       11    60  quick_shot:63 barbed_arrow:36
hunter    4 mirepinch 4    100  21.2     74      26        4    59  quick_shot:66 barbed_arrow:33
hunter    5 ashcloak 5       0   0.0    102       0       29    55  quick_shot:60 barbed_arrow:40

rogue     1 grazer 1       100   5.1     20      80        3    37  vicious_strike:74 finishing_cut:25
rogue     2 grazer 2       100   7.8     32      68        3    47  vicious_strike:63 finishing_cut:36
rogue     3 thornboar 3    100  17.0     84      16        9    62  vicious_strike:49 low_blow:27 finishing_cut:22
rogue     4 mirepinch 4      6  16.9    101       1       16    62  vicious_strike:55 low_blow:29 finishing_cut:15
rogue     5 ashcloak 5       0   0.0    101       0       28    57  vicious_strike:60 low_blow:20 finishing_cut:20

priest    1 grazer 1       100   7.8      0     100       45     0  smite:100
priest    2 grazer 2       100   9.4      0     100       29     0  smite:66 withering_word:33
priest    3 thornboar 3    100  12.3     60      40       22     0  smite:74 withering_word:25
priest    4 mirepinch 4    100  12.6     39      61       29     0  smite:75 withering_word:24
priest    5 ashcloak 5     100  14.0     91      87       26     0  smite:65 withering_word:23 mend:11

mage      1 grazer 1       100   8.2      0     100       52     0  fireball:100
mage      2 grazer 2       100   7.7      0     100       57     0  fireball:74 fire_blast:25
mage      3 thornboar 3    100  10.8     18      82       48     0  fireball:81 fire_blast:18
mage      4 mirepinch 4    100  11.9      0     100       47     0  fireball:83 fire_blast:16
mage      5 ashcloak 5     100  11.4     27      73       54     0  fireball:82 fire_blast:17

lv1: mean ttk 8.4  spread +5% -1% +45% -39% -8% -2%   taken 39 30 0 20 0 0
lv2: mean ttk 9.4  spread +4% +2% +28% -17% +1% -18%   taken 47 41 0 32 0 0
lv3: mean ttk 15.6  spread -13% +32% +23% +9% -21% -31%   taken 101 131 73 84 60 18
lv4: mean ttk 18.4  spread -26% +86% +15% -8% -32% -35%   taken 83 221 74 101 39 0
lv5: mean ttk 10.5  spread +10% +149% -100% -100% +33% +8%   taken 97 211 102 101 91 27
```

## After, one creature

```
class    lv mob           win%   ttk taken% hp left pow left idle%  casts
warrior   1 grazer 1       100   2.7      3      97        9     5  heroic_blow:57 onrush:42
warrior   2 grazer 2       100   3.0      4      96        3     1  heroic_blow:64 onrush:35
warrior   3 thornboar 3    100   4.7     15      85        4     0  heroic_blow:48 onrush:25 rending_cut:25
warrior   4 mirepinch 4    100   8.2     33      67        6    25  heroic_blow:34 onrush:21 ground_slam:21 rending_cut:21
warrior   5 ashcloak 5     100   7.3     36      64        5    15  heroic_blow:37 onrush:20 ground_slam:20 rending_cut:20

paladin   1 grazer 1       100   3.0      8      92       90    48  judging_blow:100
paladin   2 grazer 2       100   3.0      9      91       90    48  judging_blow:100
paladin   3 thornboar 3    100   5.3      8      92       69    37  judging_blow:64 rebuke:35
paladin   4 mirepinch 4    100   6.4     18      82       72    39  judging_blow:66 rebuke:33
paladin   5 ashcloak 5     100   6.4     20      80       75    39  judging_blow:66 rebuke:33

hunter    1 grazer 1       100   4.4      0     100       66    27  quick_shot:64 barbed_arrow:35
hunter    2 grazer 2       100   3.9      0     100       67    16  marked_shot:44 quick_shot:44 barbed_arrow:11
hunter    3 thornboar 3    100   6.4      0     100       53    23  quick_shot:49 marked_shot:25 barbed_arrow:25
hunter    4 mirepinch 4    100   8.5      0     100       37    18  quick_shot:39 marked_shot:20 snare_shot:20 barbed_arrow:20
hunter    5 ashcloak 5     100   7.6      0     100       46    18  quick_shot:38 marked_shot:20 snare_shot:20 barbed_arrow:20

rogue     1 grazer 1       100   2.5      7      93       11    12  vicious_strike:68 finishing_cut:31
rogue     2 grazer 2       100   3.5     11      89       14    26  vicious_strike:68 finishing_cut:25 ambush:6
rogue     3 thornboar 3    100   5.2      9      91        7    24  finishing_cut:31 low_blow:23 ambush:22 vicious_strike:22
rogue     4 mirepinch 4    100   7.7     18      82       14    37  finishing_cut:38 vicious_strike:21 low_blow:20 ambush:19
rogue     5 ashcloak 5     100   6.5     15      85        5    34  finishing_cut:37 vicious_strike:21 low_blow:20 ambush:20

priest    1 grazer 1       100   4.7      0     100       70     0  smite:100
priest    2 grazer 2       100   5.8      0     100       70     0  smite:73 withering_word:26
priest    3 thornboar 3    100   6.4     12      88       64     0  smite:75 withering_word:24
priest    4 mirepinch 4    100   7.8      0     100       63     0  smite:79 withering_word:20
priest    5 ashcloak 5     100   6.5     16      84       71     0  smite:74 withering_word:24 ward_of_light:1

mage      1 grazer 1       100   5.1      0     100       71     0  fireball:62 frostbolt:37
mage      2 grazer 2       100   3.7      0     100       73     0  frostbolt:36 fireball:36 fire_blast:26
mage      3 thornboar 3    100   6.6      0     100       67     0  fireball:47 frostbolt:26 fire_blast:26
mage      4 mirepinch 4    100   7.2      0     100       67     0  fireball:49 frostbolt:25 fire_blast:25
mage      5 ashcloak 5     100   6.7      0     100       72     0  fireball:47 frostbolt:26 fire_blast:26

lv1: mean ttk 3.7  spread -28% -19% +17% -33% +25% +37%   taken 3 8 0 7 0 0
lv2: mean ttk 3.8  spread -21% -21% +2% -9% +52% -2%   taken 4 9 0 11 0 0
lv3: mean ttk 5.7  spread -18% -9% +11% -9% +11% +14%   taken 15 8 0 9 12 0
lv4: mean ttk 7.6  spread +7% -16% +11% +1% +2% -5%   taken 33 18 0 18 0 0
lv5: mean ttk 6.9  spread +7% -7% +12% -5% -5% -2%   taken 36 20 0 15 16 0
```

## After, two creatures at once

```
class    lv mob           win%   ttk taken% hp left pow left idle%  casts
warrior   1 grazer 1       100   7.9      9      91        8    41  heroic_blow:70 onrush:29
warrior   2 grazer 2       100   8.6     13      87        3    44  heroic_blow:74 onrush:25
warrior   3 thornboar 3    100  12.9     61      57       11    25  heroic_blow:46 rending_cut:30 onrush:15 second_wind:7
warrior   4 mirepinch 4    100  16.2     81      59        6    16  heroic_blow:28 rending_cut:20 onrush:20 ground_slam:20 second_wind:10
warrior   5 ashcloak 5     100  15.0     97      44       16     9  heroic_blow:30 rending_cut:20 onrush:19 ground_slam:18 second_wind:10

paladin   1 grazer 1       100   7.9     31      69       71    60  judging_blow:100
paladin   2 grazer 2       100   7.9     34      66       71    60  judging_blow:100
paladin   3 thornboar 3    100  13.6     74      88       33    40  judging_blow:62 rebuke:19 holy_mend:18
paladin   4 mirepinch 4    100  16.7    108      60       31    42  judging_blow:68 rebuke:15 holy_mend:15
paladin   5 ashcloak 5     100  18.0    125      59       22    38  judging_blow:56 holy_mend:15 rebuke:14 devout_ward:14

hunter    1 grazer 1       100  10.0      0     100       40    25  quick_shot:60 barbed_arrow:40
hunter    2 grazer 2       100   9.4      0     100       31    23  quick_shot:57 barbed_arrow:23 marked_shot:19
hunter    3 thornboar 3    100  14.2     30      70       10    22  quick_shot:49 barbed_arrow:25 marked_shot:12 hamstring:12
hunter    4 mirepinch 4    100  20.9     49      51        6    41  quick_shot:37 barbed_arrow:24 marked_shot:12 snare_shot:12 hamstring:12
hunter    5 ashcloak 5     100  17.2     67      33        9    23  quick_shot:43 barbed_arrow:22 hamstring:11 marked_shot:11 snare_shot:11

rogue     1 grazer 1       100   9.8     36      64       15    51  vicious_strike:62 finishing_cut:37
rogue     2 grazer 2       100  11.4     50      50       18    56  vicious_strike:64 finishing_cut:28 ambush:6
rogue     3 thornboar 3    100  17.1     84      16       21    57  finishing_cut:31 vicious_strike:29 low_blow:25 ambush:13
rogue     4 mirepinch 4     96  20.5     85      15        6    54  finishing_cut:29 vicious_strike:24 low_blow:20 ambush:15 slip_away:10
rogue     5 ashcloak 5      82  19.9     99       2        8    54  finishing_cut:28 vicious_strike:27 low_blow:20 ambush:12 slip_away:10

priest    1 grazer 1       100   9.3      0     100       42     0  smite:100
priest    2 grazer 2       100  10.9      0     100       40     0  smite:71 withering_word:28
priest    3 thornboar 3    100  14.0     25      75       17     0  smite:66 withering_word:22 ward_of_light:11
priest    4 mirepinch 4    100  15.4     26      74       22     0  smite:69 withering_word:20 ward_of_light:10
priest    5 ashcloak 5     100  15.5     53      90       21     0  smite:59 withering_word:20 ward_of_light:10 renewal:10

mage      1 grazer 1       100  10.1      0     100       43     0  fireball:62 frostbolt:37
mage      2 grazer 2       100   9.9      0     100       46     0  fireball:45 frostbolt:36 fire_blast:18
mage      3 thornboar 3    100  12.3     40      60       42     0  fireball:65 frostbolt:18 fire_blast:15
mage      4 mirepinch 4    100  13.0     25      75       42     0  fireball:56 frostbolt:28 fire_blast:15
mage      5 ashcloak 5     100  12.8     55      45       50     0  fireball:67 frostbolt:17 fire_blast:14

lv1: mean ttk 9.1  spread -14% -14% +9% +7% +1% +10%   taken 9 31 0 36 0 0
lv2: mean ttk 9.7  spread -12% -18% -3% +18% +12% +2%   taken 13 34 0 50 0 0
lv3: mean ttk 14.0  spread -8% -3% +1% +22% -0% -12%   taken 61 74 30 84 25 40
lv4: mean ttk 17.1  spread -5% -3% +22% +20% -10% -24%   taken 81 108 49 85 26 25
lv5: mean ttk 16.4  spread -9% +10% +5% +21% -6% -22%   taken 97 125 67 99 53 55
```
