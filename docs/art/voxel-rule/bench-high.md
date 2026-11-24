# dyno bench

- build: dyno 0.11.1 (release)
- adapter: NVIDIA GeForce RTX 3080 (Vulkan)
- picture: 1920x1080
- world: a fresh local world of the pack's layout
- load average at the start: 9.20 7.18 6.95

| tier | graphics | ground cover | leaf detail | life | water |
|---|---|---|---|---|---|
| high (default) | 2 | 1 | 1 | 2 | 2 |

## high

| scene | GPU ms (median) | GPU ms (95%) | CPU ms (median) | CPU ms (95%) | wall ms (median) | draw calls | triangles | cover triangles | vertex memory MB | texture memory MB | mesh ms a section (CPU mean / 95%) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 1.33 | 2.55 | 2.11 | 3.45 | 2.16 | 1392 | 3.71 M | 0.40 M | 108 | 103 | 0.95 / 4.88 |
| logging_camp | 1.72 | 3.03 | 2.30 | 2.97 | 2.33 | 1370 | 5.05 M | 0.84 M | 142 | 103 | 1.41 / 6.45 |
| hollow_oak | 1.87 | 2.97 | 2.43 | 3.05 | 2.49 | 1362 | 5.73 M | 1.42 M | 148 | 103 | 1.35 / 6.76 |
| windmill_farm | 1.75 | 3.14 | 2.95 | 4.36 | 3.04 | 1136 | 3.22 M | 0.77 M | 96 | 103 | 1.13 / 4.46 |
| mirrormere | 1.26 | 2.41 | 1.66 | 4.41 | 1.68 | 1132 | 2.43 M | 0.28 M | 76 | 103 | 1.07 / 4.38 |
| miners_rest | 1.34 | 1.66 | 1.42 | 1.78 | 1.43 | 1203 | 2.60 M | 0.76 M | 63 | 103 | 0.79 / 2.93 |

By pass (median GPU ms / main-thread CPU ms / draw calls / triangles); best: the least GPU ms of any frame; setup and submit: the renderer's CPU ms before the first pass and handing the frame over; moving: vertices made anew each frame (figures, effects) and the CPU ms of setup spent uploading them; life: the ambient life's CPU ms:

| scene | best GPU | setup | submit | moving | life | shadow near | shadow mid | shadow far | world | occlusion | sky | water | translucent | post | interface |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| hearthstead_plaza | 1.29 | 0.91 | 0.59 | 107 k / 0.82 | 0.31 | 0.06 / 0.04 / 20 / 0.17 M | 0.18 / 0.04 / 104 / 0.78 M | 0.16 / 0.05 / 254 / 1.15 M | 0.64 / 0.10 / 808 / 1.61 M | 0.14 / 0.00 / 2 / 2 | 0.04 / 0.00 / 1 / 1 | 0.03 / 0.02 / 64 / 1848 | 0.01 / 0.01 / 129 / 2158 | 0.07 / 0.01 / 10 / 10 | 0.00 / 0.00 / 0 / 0 |
| logging_camp | 1.42 | 0.56 | 0.96 | 51 k / 0.45 | 0.31 | 0.07 / 0.08 / 23 / 0.28 M | 0.20 / 0.06 / 98 / 0.74 M | 0.24 / 0.09 / 313 / 1.59 M | 0.85 / 0.17 / 910 / 2.45 M | 0.18 / 0.00 / 2 / 2 | 0.02 / 0.00 / 1 / 1 | 0.07 / 0.02 / 3 / 426 | 0.01 / 0.01 / 10 / 456 | 0.09 / 0.01 / 10 / 10 | 0.00 / 0.01 / 0 / 0 |
| hollow_oak | 1.52 | 0.85 | 0.84 | 63 k / 0.72 | 0.23 | 0.13 / 0.09 / 26 / 0.32 M | 0.21 / 0.07 / 109 / 0.81 M | 0.24 / 0.09 / 289 / 1.69 M | 0.66 / 0.16 / 901 / 2.91 M | 0.14 / 0.00 / 2 / 2 | 0.03 / 0.00 / 1 / 1 | 0.06 / 0.02 / 4 / 522 | 0.01 / 0.01 / 20 / 786 | 0.07 / 0.01 / 10 / 10 | 0.00 / 0.01 / 0 / 0 |
| windmill_farm | 1.08 | 0.97 | 0.97 | 64 k / 0.82 | 0.39 | 0.08 / 0.10 / 14 / 0.18 M | 0.17 / 0.08 / 66 / 0.49 M | 0.16 / 0.09 / 199 / 0.80 M | 0.78 / 0.22 / 793 / 1.74 M | 0.18 / 0.01 / 2 / 2 | 0.03 / 0.00 / 1 / 1 | 0.08 / 0.02 / 19 / 2302 | 0.01 / 0.01 / 32 / 2348 | 0.11 / 0.01 / 10 / 10 | 0.00 / 0.01 / 0 / 0 |
| mirrormere | 1.22 | 0.50 | 0.61 | 45 k / 0.41 | 0.19 | 0.17 / 0.06 / 59 / 0.31 M | 0.12 / 0.04 / 142 / 0.49 M | 0.07 / 0.04 / 211 / 0.59 M | 0.38 / 0.10 / 553 / 0.98 M | 0.13 / 0.00 / 2 / 2 | 0.02 / 0.00 / 1 / 1 | 0.27 / 0.02 / 71 / 29746 | 0.02 / 0.01 / 83 / 29800 | 0.07 / 0.01 / 10 / 10 | 0.00 / 0.00 / 0 / 0 |
| miners_rest | 1.29 | 0.42 | 0.51 | 47 k / 0.36 | 0.22 | 0.08 / 0.04 / 26 / 0.25 M | 0.10 / 0.03 / 91 / 0.37 M | 0.08 / 0.04 / 235 / 0.53 M | 0.66 / 0.10 / 792 / 1.45 M | 0.15 / 0.00 / 2 / 2 | 0.01 / 0.00 / 1 / 1 | 0.16 / 0.01 / 18 / 1018 | 0.01 / 0.00 / 28 / 1086 | 0.07 / 0.01 / 10 / 10 | 0.00 / 0.00 / 0 / 0 |
