//! Armour: item sets by slot (chest, shoulders, head, hands, waist, legs,
//! feet, back), each in cloth, leather, mail and plate, coloured by the
//! item through `<slot>.main`, `<slot>.trim` and `<slot>.accent`.

use super::body::Body;
use super::person::Inks;
use super::{Grid, Palette};

pub fn all(_main: &mut Grid, _head: &mut Grid, _bodies: &[Body; 2], _ink: &Inks, _pal: &mut Palette) {}

pub fn helms(_head: &mut Grid, _bodies: &[Body; 2], _ink: &Inks, _pal: &mut Palette) {}
