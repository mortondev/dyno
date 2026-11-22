use super::*;
pub fn build_hearthstead(_world: &mut World, _hearth: [i32; 3]) -> bool { false }
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn survey() {
        for seed in [20261122u32, 1, 2, 3, 42, 777] {
            let mut w = World::generate([256, 96, 256], seed);
            let h = w.ensure_spawn_hearth(None);
            let y0 = h[1] - 1;
            let tree = |b| matches!(b, LOG | LEAVES | 34 | 35 | 36 | BERRY_BUSH | GLOWSHROOM);
            let (mut lo, mut hi, mut odd, mut water) = (99, -99, 0, 0);
            let mut rows = vec![];
            for dz in (-38..=38).step_by(4) {
                let mut row = String::new();
                for dx in (-38..=38).step_by(2) {
                    let (x, z) = (h[0] + dx, h[2] + dz);
                    let mut g = 95;
                    while g > 0 && (!is_solid(w.get(x, g, z)) || tree(w.get(x, g, z))) { g -= 1; }
                    if w.get(x, g + 1, z) == WATER { water += 1; }
                    let d = g - y0;
                    lo = lo.min(d); hi = hi.max(d);
                    row.push_str(&format!("{:>3}", d));
                }
                rows.push(row);
            }
            for dz in -38..=38 { for dx in -38..=38 { for y in y0 + 1..96 {
                let b = w.get(h[0] + dx, y, h[2] + dz);
                if !matches!(b, AIR | GRASS | DIRT | STONE | SAND | SNOW | LOG | LEAVES | 33..=36 | BERRY_BUSH | WATER) && dx*dx+dz*dz > 64 { odd += 1; eprint!("[{}@{},{},{}]", b, dx, y - y0, dz); }
            }}}
            eprintln!("\nseed {seed} hearth {h:?} range {lo}..{hi} odd {odd} water {water}");
            for r in rows { eprintln!("{r}"); }
        }
    }
}
