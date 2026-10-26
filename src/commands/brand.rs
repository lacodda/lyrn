//! `lyrn brand pick`: the brand-line ritual's checks, as a command.
//!
//! A new product of the line gets a two-letter code and a colour - one hue,
//! or a pair drawn as one gradient - and the owner picks them on a board. What
//! the board cannot show by looking is what this holds them to: a code
//! another product has, a hue too close to a neighbour's, a pair already taken
//! in that direction, and a code on a colour that is somebody else's mark.
//! That last one was caught by the owner's eye, once, after the assets were
//! laid out: `mi` on orange is Xiaomi. Here it is a table.
//!
//! The registry is the marks dowel-ui publishes, vendored into lyrn by
//! `tools/vendor-dowel.mjs`: each product's code and colours as its S master
//! draws them.

use std::error::Error;
use std::path::Path;

use crate::cli::BrandPickArgs;
use crate::repo::Facts;

/// The line's marks: product, code, colour, and the second colour of a pair.
const REGISTRY: &str = include_str!("../templates/dowel/marks.tsv");

/// A single colour needs this much hue between it and every colour of the
/// line; closer, it is only fit to be the second colour of a pair.
const SINGLE_GAP: f32 = 12.0;

/// Below this saturation a colour has no hue to speak of: a grey, and the
/// umbrella mark's graphite is the line's one grey.
const ACHROMATIC: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(u8, u8, u8);

impl Rgb {
    pub fn parse(hex: &str) -> Option<Rgb> {
        let digits = hex.trim().strip_prefix('#')?;
        if digits.len() != 6 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let byte = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
        Some(Rgb(byte(0)?, byte(2)?, byte(4)?))
    }

    fn hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    /// Hue in degrees and saturation, by HSL.
    fn hue_saturation(self) -> (f32, f32) {
        let [r, g, b] = [self.0, self.1, self.2].map(|c| f32::from(c) / 255.0);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        if delta == 0.0 {
            return (0.0, 0.0);
        }
        let lightness = (max + min) / 2.0;
        let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
        let hue = if max == r {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        (hue, saturation)
    }

    /// The hue, when the colour has one.
    pub fn hue(self) -> Option<f32> {
        let (hue, saturation) = self.hue_saturation();
        (saturation >= ACHROMATIC).then_some(hue)
    }

    /// A colour of this hue at the saturation and lightness the line's
    /// colours sit around, to show a free hue as something to try.
    fn of_hue(hue: f32) -> Rgb {
        let (s, l) = (0.62_f32, 0.55_f32);
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let x = c * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
        let m = l - c / 2.0;
        let (r, g, b) = match hue as u32 / 60 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let byte = |v: f32| ((v + m) * 255.0).round() as u8;
        Rgb(byte(r), byte(g), byte(b))
    }
}

/// Degrees between two hues, the short way round.
fn gap(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

/// A mark of the line, as the registry states it.
#[derive(Debug, Clone)]
pub struct Mark {
    pub name: String,
    pub code: String,
    pub colours: Vec<Rgb>,
}

pub fn registry() -> Vec<Mark> {
    REGISTRY
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.to_string();
            let code = fields.next()?.to_string();
            let colours: Vec<Rgb> = fields.filter_map(Rgb::parse).collect();
            (!colours.is_empty()).then_some(Mark { name, code, colours })
        })
        .collect()
}

/// A known mark that is a code on a colour: the code, the hues it covers
/// (from, to, clockwise), and whether it fails or only warns. A warning is a
/// resemblance the owner may accept, and has: `ln` on indigo, `rh` on green.
struct Known {
    code: &'static str,
    hues: (f32, f32),
    fails: bool,
    mark: &'static str,
}

/// Two letters on a colour that already mean something. The first ones were
/// each caught at the line's board, by eye; Adobe's are here because its app
/// icons are the line's own pattern - two letters on a dark tile, one colour
/// per product - and a product of the line wearing one would read as Adobe's.
const KNOWN: &[Known] = &[
    Known {
        code: "mi",
        hues: (10.0, 45.0),
        fails: true,
        mark: "Xiaomi: mi in white on orange",
    },
    Known {
        code: "ef",
        hues: (340.0, 25.0),
        fails: true,
        mark: "EF Education First: white letters on red",
    },
    Known {
        code: "rh",
        hues: (340.0, 20.0),
        fails: true,
        mark: "Red Hat: red, with RH",
    },
    Known {
        code: "rh",
        hues: (90.0, 150.0),
        fails: false,
        mark: "Robinhood: green, RH",
    },
    Known {
        code: "ks",
        hues: (90.0, 160.0),
        fails: true,
        mark: "Kickstarter: green",
    },
    Known {
        code: "au",
        hues: (40.0, 160.0),
        fails: false,
        mark: "Australia: green and gold, .au",
    },
    Known {
        code: "rr",
        hues: (340.0, 20.0),
        fails: false,
        mark: "Rolls-Royce: RR on red",
    },
    Known {
        code: "dw",
        hues: (200.0, 250.0),
        fails: true,
        mark: "Deutsche Welle: DW on blue",
    },
    Known {
        code: "in",
        hues: (195.0, 240.0),
        fails: true,
        mark: "LinkedIn: in on blue",
    },
    Known {
        code: "ln",
        hues: (195.0, 240.0),
        fails: false,
        mark: "LinkedIn: at 16px, ln reads as in on blue",
    },
    Known {
        code: "fc",
        hues: (90.0, 160.0),
        fails: false,
        mark: "a football club's crest: FC on green",
    },
    Known {
        code: "ly",
        hues: (300.0, 345.0),
        fails: true,
        mark: "Lyft: pink",
    },
    Known {
        code: "ps",
        hues: (190.0, 220.0),
        fails: true,
        mark: "Adobe Photoshop: Ps on a dark tile, blue",
    },
    Known {
        code: "lr",
        hues: (190.0, 220.0),
        fails: true,
        mark: "Adobe Lightroom: Lr on a dark tile, blue",
    },
    Known {
        code: "ai",
        hues: (22.0, 50.0),
        fails: true,
        mark: "Adobe Illustrator: Ai on a dark tile, orange",
    },
    Known {
        code: "id",
        hues: (330.0, 360.0),
        fails: true,
        mark: "Adobe InDesign: Id on a dark tile, pink",
    },
    Known {
        code: "ae",
        hues: (225.0, 255.0),
        fails: true,
        mark: "Adobe After Effects: Ae on a dark tile, violet",
    },
    Known {
        code: "pr",
        hues: (225.0, 255.0),
        fails: true,
        mark: "Adobe Premiere Pro: Pr on a dark tile, violet",
    },
    Known {
        code: "xd",
        hues: (290.0, 318.0),
        fails: true,
        mark: "Adobe XD: Xd on a dark tile, magenta",
    },
    Known {
        code: "au",
        hues: (155.0, 185.0),
        fails: true,
        mark: "Adobe Audition: Au on a dark tile, teal",
    },
    Known {
        code: "dw",
        hues: (92.0, 122.0),
        fails: true,
        mark: "Adobe Dreamweaver: Dw on a dark tile, green",
    },
    Known {
        code: "an",
        hues: (0.0, 30.0),
        fails: true,
        mark: "Adobe Animate: An on a dark tile, red-orange",
    },
];

/// Codes that read as something before they read as a product.
const READS_AS: &[(&str, &str)] = &[
    ("hi", "reads as a word"),
    ("fu", "reads as a word"),
    ("ok", "reads as a word"),
    ("no", "reads as a word"),
    ("so", "reads as a word"),
    ("do", "reads as a word"),
    ("go", "reads as a word, and is a language"),
    ("up", "reads as a word"),
    ("sd", "reads as an SD card"),
    ("sh", "reads as the shell"),
    ("rg", "is ripgrep's name"),
    ("js", "is JavaScript"),
    ("ts", "is TypeScript"),
    ("ui", "reads as a user interface"),
    ("io", "reads as input and output"),
    ("ai", "reads as artificial intelligence"),
    ("os", "reads as an operating system"),
    ("db", "reads as a database"),
    ("tv", "reads as television"),
    ("pc", "reads as a computer"),
    ("vs", "reads as versus, and is Visual Studio"),
];

/// One line of the verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Free(String),
    Warn(String),
    Taken(String),
}

impl Verdict {
    fn mark(&self) -> &'static str {
        match self {
            Verdict::Free(_) => "✓",
            Verdict::Warn(_) => "!",
            Verdict::Taken(_) => "✗",
        }
    }

    fn text(&self) -> &str {
        match self {
            Verdict::Free(t) | Verdict::Warn(t) | Verdict::Taken(t) => t,
        }
    }
}

/// The codes the line's rule makes of a name: its first letter and a
/// consonant after it, or the first letter of each word.
pub fn candidates(name: &str) -> Vec<String> {
    let words: Vec<&str> = name.split('-').filter(|w| !w.is_empty()).collect();
    let Some(first) = words.first().and_then(|w| w.chars().next()).filter(char::is_ascii_lowercase) else {
        return Vec::new();
    };
    let mut out: Vec<String> = Vec::new();
    let mut push = |second: char| {
        let code = format!("{first}{second}");
        if second.is_ascii_lowercase() && !out.contains(&code) {
            out.push(code);
        }
    };
    for word in words.iter().skip(1) {
        if let Some(c) = word.chars().next() {
            push(c);
        }
    }
    for c in name.chars().skip(1).filter(|c| c.is_ascii_lowercase() && !"aeiouy".contains(*c)) {
        push(c);
    }
    // The second letter, vowel or not: `se`, `mi` and `ki` are the line's too.
    if let Some(c) = words[0].chars().nth(1) {
        push(c);
    }
    out
}

/// What a code is, against the registry and what codes read as.
pub fn code_verdict(code: &str, name: &str, marks: &[Mark]) -> Verdict {
    if let Some(mark) = marks.iter().find(|m| m.code == code && m.name != name) {
        return Verdict::Taken(format!("{}'s", mark.name));
    }
    if let Some((_, why)) = READS_AS.iter().find(|(c, _)| *c == code) {
        return Verdict::Warn(why.to_string());
    }
    Verdict::Free("free".to_string())
}

/// What a single colour is, against every colour of the line.
pub fn colour_verdict(colour: Rgb, name: &str, marks: &[Mark]) -> Verdict {
    let Some(hue) = colour.hue() else {
        return Verdict::Taken("a grey: graphite is the umbrella mark's, and a grey reads as no colour".to_string());
    };
    let others: Vec<(&Mark, Rgb, f32)> = marks
        .iter()
        .filter(|m| m.name != name)
        .flat_map(|m| m.colours.iter().filter_map(move |c| c.hue().map(|h| (m, *c, h))))
        .collect();
    if let Some((mark, _, _)) = others.iter().find(|(m, c, _)| *c == colour && m.colours.len() == 1) {
        return Verdict::Taken(format!("{}'s colour", mark.name));
    }
    let mut nearest: Vec<&(&Mark, Rgb, f32)> = others.iter().collect();
    nearest.sort_by(|a, b| gap(a.2, hue).total_cmp(&gap(b.2, hue)));
    let named: Vec<String> = nearest
        .iter()
        .take(2)
        .map(|(m, c, h)| format!("{:.0}° from {} {} ({h:.0}°)", gap(*h, hue), m.name, c.hex()))
        .collect();
    match nearest.first() {
        Some((_, _, h)) if gap(*h, hue) < SINGLE_GAP => Verdict::Taken(format!(
            "{}: a single colour needs {SINGLE_GAP:.0}° - this one is fit for the second colour of a pair",
            named.join(" and ")
        )),
        _ => Verdict::Free(format!("{}: free for a single colour", named.join(" and "))),
    }
}

/// What a pair is: any colour of the palette is open to a pair, and only the
/// same two in the same direction are taken.
pub fn pair_verdict(from: Rgb, to: Rgb, name: &str, marks: &[Mark]) -> Verdict {
    if from == to {
        return Verdict::Taken("a pair of one colour is a single colour".to_string());
    }
    if from.hue().is_none() || to.hue().is_none() {
        return Verdict::Taken("a grey in the pair: graphite is the umbrella mark's, and a grey reads as no colour".to_string());
    }
    let others = marks.iter().filter(|m| m.name != name && m.colours.len() == 2);
    for mark in others {
        if mark.colours == [from, to] {
            return Verdict::Taken(format!("{}'s pair, in the same direction", mark.name));
        }
        if mark.colours == [to, from] {
            return Verdict::Free(format!("the reverse of {}'s pair - a pair runs one way, so it is free", mark.name));
        }
    }
    Verdict::Free("free: a pair may take any colour of the palette".to_string())
}

/// What the code is on these colours: a known mark it would read as.
pub fn known_verdicts(code: &str, colours: &[Rgb]) -> Vec<Verdict> {
    let mut out = Vec::new();
    for colour in colours {
        let Some(hue) = colour.hue() else { continue };
        for known in KNOWN.iter().filter(|k| k.code == code) {
            let (from, to) = known.hues;
            let inside = if from <= to { (from..=to).contains(&hue) } else { hue >= from || hue <= to };
            if inside {
                let text = format!("{code} on {} ({hue:.0}°) is {}", colour.hex(), known.mark);
                out.push(if known.fails { Verdict::Taken(text) } else { Verdict::Warn(text) });
            }
        }
    }
    out
}

/// The hues no colour of the line is near, widest first: the middle of each
/// arc between two of the line's hues that leaves a single colour its gap on
/// both sides.
pub fn free_hues(marks: &[Mark]) -> Vec<(f32, f32)> {
    let mut hues: Vec<f32> = marks.iter().flat_map(|m| m.colours.iter().filter_map(|c| c.hue())).collect();
    hues.sort_by(f32::total_cmp);
    hues.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    let mut out: Vec<(f32, f32)> = Vec::new();
    for (i, &here) in hues.iter().enumerate() {
        let next = if i + 1 < hues.len() { hues[i + 1] } else { hues[0] + 360.0 };
        let width = next - here;
        if width >= 2.0 * SINGLE_GAP {
            out.push(((here + width / 2.0) % 360.0, width / 2.0));
        }
    }
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out
}

pub fn pick(args: BrandPickArgs) -> Result<bool, Box<dyn Error>> {
    let name = match &args.name {
        Some(name) => name.clone(),
        None => Facts::read(Path::new("."))?
            .name
            .or_else(|| std::env::current_dir().ok()?.file_name().map(|n| n.to_string_lossy().into_owned()))
            .ok_or("name the product: `lyrn brand pick <name>`")?,
    };
    let parse = |hex: &Option<String>| -> Result<Option<Rgb>, String> {
        hex.as_deref()
            .map(|h| Rgb::parse(h).ok_or_else(|| format!("`{h}` is not a `#rrggbb` colour")))
            .transpose()
    };
    let colour = parse(&args.colour)?;
    let to = parse(&args.to)?;
    if let Some(code) = &args.code
        && !(code.len() == 2 && code.chars().all(|c| c.is_ascii_lowercase()))
    {
        return Err(format!("`{code}` is not a code: two lowercase letters").into());
    }

    let marks = registry();
    println!("lyrn brand pick: {name}, against the line's {} marks\n", marks.len());
    let mut passed = true;
    let mut say = |verdict: &Verdict, label: &str| {
        if matches!(verdict, Verdict::Taken(_)) {
            passed = false;
        }
        println!("  {} {label}{}", verdict.mark(), verdict.text());
    };

    println!("code");
    let codes: Vec<String> = match &args.code {
        Some(code) => vec![code.clone()],
        None => candidates(&name),
    };
    for code in &codes {
        let verdict = code_verdict(code, &name, &marks);
        // Without a code chosen the list is a menu, and a taken one is only
        // a choice not to make.
        if args.code.is_none() && matches!(verdict, Verdict::Taken(_)) {
            println!("  ✗ {code}  {}", verdict.text());
        } else {
            say(&verdict, &format!("{code}  "));
        }
    }

    let colours: Vec<Rgb> = colour.into_iter().chain(to).collect();
    match (colour, to) {
        (Some(single), None) => {
            println!("\ncolour {}", single.hex());
            say(&colour_verdict(single, &name, &marks), "");
        }
        (Some(from), Some(to)) => {
            println!("\npair {} → {}", from.hex(), to.hex());
            say(&pair_verdict(from, to, &name, &marks), "");
        }
        (None, Some(_)) => return Err("`--to` is the second colour of a pair: give the first with `--colour`".into()),
        (None, None) => {}
    }

    if !colours.is_empty() {
        let shown = &codes[..];
        let mut any = false;
        for code in shown {
            for verdict in known_verdicts(code, &colours) {
                if !any {
                    println!("\ncode on colour");
                    any = true;
                }
                if args.code.is_some() {
                    say(&verdict, "");
                } else {
                    println!("  {} {}", verdict.mark(), verdict.text());
                }
            }
        }
        if !any {
            let which: Vec<&str> = shown.iter().map(String::as_str).collect();
            println!(
                "\ncode on colour\n  ✓ no known mark is {} on {}",
                which.join(", "),
                colours.iter().map(|c| c.hex()).collect::<Vec<_>>().join(" or ")
            );
        }
    }

    let free = free_hues(&marks);
    if free.is_empty() {
        println!("\nNo hue is free for a single colour: a new mark takes a pair.");
    } else {
        let listed: Vec<String> = free
            .iter()
            .take(4)
            .map(|(hue, room)| format!("{hue:.0}° {} ({room:.0}° clear)", Rgb::of_hue(*hue).hex()))
            .collect();
        println!("\nfree for a single colour: {}", listed.join(", "));
    }
    Ok(passed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(hex: &str) -> Rgb {
        Rgb::parse(hex).unwrap()
    }

    #[test]
    fn the_registry_is_every_mark_of_the_line() {
        let marks = registry();
        assert!(marks.len() >= 20, "{} marks", marks.len());
        let lyrn = marks.iter().find(|m| m.name == "lyrn").unwrap();
        assert_eq!((lyrn.code.as_str(), lyrn.colours.as_slice()), ("ln", &[rgb("#6D7BF2")][..]));
        let rigger = marks.iter().find(|m| m.name == "rigger").unwrap();
        assert_eq!(rigger.colours, [rgb("#8A62F0"), rgb("#2FB3C6")], "a pair, in the order its gradient runs");
        // Every product's accent is its mark's first colour: two lists from
        // one package that cannot disagree.
        for (product, accent) in crate::naming::line_accents() {
            let mark = marks.iter().find(|m| m.name == product).unwrap_or_else(|| panic!("{product} has no mark"));
            assert_eq!(mark.colours[0], rgb(accent), "{product}");
        }
    }

    #[test]
    fn a_hue_is_read_as_the_board_reads_it() {
        assert_eq!(rgb("#FF0000").hue(), Some(0.0));
        assert_eq!(rgb("#00FF00").hue(), Some(120.0));
        assert!((rgb("#3EB556").hue().unwrap() - 132.0).abs() < 1.0, "grass");
        assert_eq!(rgb("#8E9AA3").hue(), None, "graphite has no hue to speak of");
        assert_eq!(gap(350.0, 10.0), 20.0);
        let again = Rgb::of_hue(132.0).hue().unwrap();
        assert!((again - 132.0).abs() < 2.0, "{again}");
    }

    #[test]
    fn codes_come_from_the_name_by_the_line_s_rule() {
        assert_eq!(candidates("hilvan")[..3], ["hl", "hv", "hn"]);
        assert!(candidates("hilvan").contains(&"hi".to_string()), "the second letter is offered too");
        assert_eq!(candidates("kasl-server")[0], "ks", "a second word gives its first letter");
        assert!(candidates("").is_empty());
    }

    /// The checks the owner's eye made once, made by the table: `mi` on
    /// orange is Xiaomi, the code a product has is its own.
    #[test]
    fn the_lessons_of_the_board_are_caught() {
        let marks = registry();
        let verdicts = known_verdicts("mi", &[rgb("#D9704A")]);
        assert!(matches!(&verdicts[..], [Verdict::Taken(t)] if t.contains("Xiaomi")), "{verdicts:?}");
        assert!(known_verdicts("mi", &[rgb("#A46BE8")]).is_empty(), "violet, as midda took it");
        assert!(matches!(code_verdict("ln", "newcomer", &marks), Verdict::Taken(t) if t == "lyrn's"));
        assert_eq!(code_verdict("ln", "lyrn", &marks), Verdict::Free("free".to_string()), "a product's own code");
        assert!(matches!(code_verdict("hi", "hilvan", &marks), Verdict::Warn(_)));
        // A hue next to a neighbour's is only fit for a pair; a free one is free.
        assert!(matches!(colour_verdict(rgb("#3FA9DA"), "newcomer", &marks), Verdict::Taken(t) if t.contains("pair")));
        assert!(matches!(colour_verdict(rgb("#8E9AA3"), "newcomer", &marks), Verdict::Taken(t) if t.contains("grey")));
        // A pair taken in one direction is free in the other.
        assert!(matches!(pair_verdict(rgb("#D9704A"), rgb("#2FAF8C"), "newcomer", &marks), Verdict::Taken(t) if t.contains("scheda")));
        assert!(matches!(pair_verdict(rgb("#2FAF8C"), rgb("#D9704A"), "newcomer", &marks), Verdict::Free(t) if t.contains("reverse")));
    }

    /// The line's own marks pass their own check: every resemblance the owner
    /// accepted is a warning, not a failure.
    #[test]
    fn every_mark_of_the_line_passes_its_own_check() {
        for mark in registry() {
            for verdict in known_verdicts(&mark.code, &mark.colours) {
                assert!(!matches!(verdict, Verdict::Taken(_)), "{}: {verdict:?}", mark.name);
            }
        }
    }

    #[test]
    fn free_hues_leave_a_single_colour_its_gap() {
        let marks = registry();
        let free = free_hues(&marks);
        for (hue, room) in &free {
            assert!(*room >= SINGLE_GAP, "{hue}: {room}");
            assert!(matches!(colour_verdict(Rgb::of_hue(*hue), "newcomer", &marks), Verdict::Free(_)), "{hue}");
        }
    }
}
