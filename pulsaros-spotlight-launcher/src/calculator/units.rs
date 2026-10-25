//! Unit table: dimension vectors, factors to a coherent base, lookup by name.

use super::currency::CurrencyTable;
use std::collections::HashMap;

/// Exponents of [length, mass, time, temperature, data, angle, currency, current].
pub type Dim = [i8; 8];
pub const NONE: Dim = [0; 8];
pub const CUR: usize = 6;

#[derive(Clone, Debug)]
pub struct Unit {
    pub sym: String,
    pub long: String,
    pub dim: Dim,
    /// Multiplier to the coherent base (m, kg, s, K, byte, rad, USD, A).
    pub f: f64,
    /// Offset in the unit's own scale (temperatures): base = (x + off) * f.
    pub off: f64,
    /// Compound expansion for derived units such as `mph` -> `mi/h`.
    pub expand: Option<&'static str>,
}

pub fn dim_of(name: &str) -> Dim {
    match name {
        "length" => [1, 0, 0, 0, 0, 0, 0, 0],
        "mass" => [0, 1, 0, 0, 0, 0, 0, 0],
        "time" => [0, 0, 1, 0, 0, 0, 0, 0],
        "temp" => [0, 0, 0, 1, 0, 0, 0, 0],
        "data" => [0, 0, 0, 0, 1, 0, 0, 0],
        "angle" => [0, 0, 0, 0, 0, 1, 0, 0],
        "current" => [0, 0, 0, 0, 0, 0, 0, 1],
        "area" => [2, 0, 0, 0, 0, 0, 0, 0],
        "volume" => [3, 0, 0, 0, 0, 0, 0, 0],
        "speed" => [1, 0, -1, 0, 0, 0, 0, 0],
        "force" => [1, 1, -2, 0, 0, 0, 0, 0],
        "energy" => [2, 1, -2, 0, 0, 0, 0, 0],
        "power" => [2, 1, -3, 0, 0, 0, 0, 0],
        "pressure" => [-1, 1, -2, 0, 0, 0, 0, 0],
        "freq" => [0, 0, -1, 0, 0, 0, 0, 0],
        "voltage" => [2, 1, -3, 0, 0, 0, 0, -1],
        "resistance" => [2, 1, -3, 0, 0, 0, 0, -2],
        "charge" => [0, 0, 1, 0, 0, 0, 0, 1],
        _ => NONE,
    }
}

// dim|sym|factor|long name|aliases[|offset]
const TABLE: &str = "\
length|m|1|Meters|meter,meters,metre,metres
length|km|1000|Kilometers|kilometer,kilometers,kilometre,kilometres
length|cm|0.01|Centimeters|centimeter,centimeters,centimetre,centimetres
length|mm|0.001|Millimeters|millimeter,millimeters,millimetre,millimetres
length|dm|0.1|Decimeters|decimeter,decimeters
length|hm|100|Hectometers|hectometer,hectometers
length|um|1e-6|Micrometers|µm,micron,microns,micrometer,micrometers
length|nm|1e-9|Nanometers|nanometer,nanometers
length|pm|1e-12|Picometers|picometer,picometers
length|mi|1609.344|Miles|mile,miles
length|yd|0.9144|Yards|yard,yards,yds
length|ft|0.3048|Feet|foot,feet,fts
length|in|0.0254|Inches|inch,inches
length|nmi|1852|Nautical miles|nauticalmile,nauticalmiles
length|ly|9460730472580800|Light years|lightyear,lightyears
length|au|149597870700|Astronomical units|astronomicalunit
length|pc|30856775814913673|Parsecs|parsec,parsecs
length|angstrom|1e-10|Angstroms|angstroms,å
length|mil|0.0000254|Mils|thou
length|furlong|201.168|Furlongs|furlongs
length|fathom|1.8288|Fathoms|fathoms
length|chain|20.1168|Chains|chains
length|league|4828.032|Leagues|leagues
length|rod|5.0292|Rods|rods
length|hand|0.1016|Hands|hands
length|link|0.201168|Links|links
length|cubit|0.4572|Cubits|cubits
length|px|0.0254/96|Pixels|pixel,pixels,pixel
length|pt|0.0254/72|Points|point,points
length|pica|0.0254/6|Picas|picas
length|rem|0.0254/96*16|Rem|rems
length|em|0.0254/96*16|Em|ems
mass|kg|1|Kilograms|kilogram,kilograms,kilo,kilos
mass|g|0.001|Grams|gram,grams
mass|mg|1e-6|Milligrams|milligram,milligrams
mass|ug|1e-9|Micrograms|µg,mcg,microgram,micrograms
mass|ng|1e-12|Nanograms|nanogram,nanograms
mass|t|1000|Metric tons|tonne,tonnes,metricton,metrictons
mass|lb|0.45359237|Pounds|lbs,pound,pounds
mass|oz|0.028349523125|Ounces|ounce,ounces
mass|st|6.35029318|Stone|stone,stones
mass|ton|907.18474|Short tons|tons,shortton,shorttons,uston
mass|longton|1016.0469088|Long tons|longtons,ukton
mass|ct|0.0002|Carats|carat,carats
mass|grain|0.00006479891|Grains|grains,gr
mass|dram|0.0017718451953125|Drams|drams
mass|slug|14.59390294|Slugs|slugs
mass|quintal|100|Quintals|quintals
mass|amu|1.66053906660e-27|Atomic mass units|dalton,daltons
time|s|1|Seconds|sec,secs,second,seconds
time|ms|0.001|Milliseconds|millisecond,milliseconds,msec
time|us|1e-6|Microseconds|µs,microsecond,microseconds
time|ns|1e-9|Nanoseconds|nanosecond,nanoseconds
time|min|60|Minutes|mins,minute,minutes
time|h|3600|Hours|hr,hrs,hour,hours
time|d|86400|Days|day,days
time|wk|604800|Weeks|week,weeks
time|fortnight|1209600|Fortnights|fortnights
time|mo|2629800|Months|month,months
time|yr|31557600|Years|y,year,years,yrs
time|decade|315576000|Decades|decades
time|century|3155760000|Centuries|centuries
time|millennium|31557600000|Millennia|millennia
time|workday|28800|Work days|workdays,workingday,workingdays,businessday,businessdays
time|workweek|144000|Work weeks|workweeks,workingweek,workingweeks
time|workmonth|624000|Work months|workmonths
time|workyear|7488000|Work years|workyears
temp|c|1|Celsius|celsius,degc,degreescelsius,degreecelsius|273.15
temp|f|5/9|Fahrenheit|fahrenheit,degf,degreesfahrenheit,degreefahrenheit|459.67
temp|k|1|Kelvin|kelvin,kelvins
area|m2|1|Square meters|sqm,squaremeter,squaremeters,squaremetre
area|km2|1e6|Square kilometers|sqkm,squarekilometer,squarekilometers
area|cm2|0.0001|Square centimeters|sqcm,squarecentimeter
area|mm2|1e-6|Square millimeters|sqmm
area|ha|10000|Hectares|hectare,hectares
area|are|100|Ares|ares
area|acre|4046.8564224|Acres|acres
area|ft2|0.09290304|Square feet|sqft,squarefoot,squarefeet
area|in2|0.00064516|Square inches|sqin,squareinch,squareinches
area|yd2|0.83612736|Square yards|sqyd,squareyard,squareyards
area|mi2|2589988.110336|Square miles|sqmi,squaremile,squaremiles
volume|l|0.001|Liters|liter,liters,litre,litres,ltr
volume|ml|1e-6|Milliliters|milliliter,milliliters,millilitre,millilitres
volume|cl|1e-5|Centiliters|centiliter,centiliters
volume|dl|1e-4|Deciliters|deciliter,deciliters
volume|kl|1|Kiloliters|kiloliter,kiloliters
volume|m3|1|Cubic meters|cubicmeter,cubicmeters,cbm
volume|cm3|1e-6|Cubic centimeters|cc,cubiccentimeter,cubiccentimeters
volume|mm3|1e-9|Cubic millimeters|cubicmillimeter
volume|km3|1e9|Cubic kilometers|cubickilometer
volume|ft3|0.028316846592|Cubic feet|cuft,cubicfoot,cubicfeet
volume|in3|0.000016387064|Cubic inches|cuin,cubicinch,cubicinches
volume|yd3|0.764554857984|Cubic yards|cuyd,cubicyard,cubicyards
volume|gal|0.003785411784|Gallons|gallon,gallons,usgal
volume|qt|0.000946352946|Quarts|quart,quarts
volume|pint|0.000473176473|Pints|pints
volume|cup|0.0002365882365|Cups|cups
volume|floz|0.0000295735295625|Fluid ounces|fluidounce,fluidounces
volume|tbsp|0.00001478676478125|Tablespoons|tablespoon,tablespoons
volume|tsp|0.00000492892159375|Teaspoons|teaspoon,teaspoons
volume|impgal|0.00454609|Imperial gallons|imperialgallon,imperialgallons,ukgal
volume|impqt|0.0011365225|Imperial quarts|imperialquart
volume|imppint|0.00056826125|Imperial pints|imperialpint
volume|bbl|0.158987294928|Barrels|barrel,barrels
speed|mph|0.44704|Miles per hour|milesperhour
speed|kph|0.2777777777778|Kilometers per hour|kmh,kilometersperhour
speed|fps|0.3048|Feet per second|feetpersecond
speed|knot|0.514444444444|Knots|knots,kn,kt,kts
speed|mach|340.29|Mach|
speed|lightspeed|299792458|Speed of light|
data|bit|0.125|Bits|bits
data|byte|1|Bytes|bytes
data|kb|1000|Kilobytes|kilobyte,kilobytes
data|mb|1e6|Megabytes|megabyte,megabytes
data|gb|1e9|Gigabytes|gigabyte,gigabytes
data|tb|1e12|Terabytes|terabyte,terabytes
data|pb|1e15|Petabytes|petabyte,petabytes
data|eb|1e18|Exabytes|exabyte,exabytes
data|kib|1024|Kibibytes|kibibyte,kibibytes
data|mib|1048576|Mebibytes|mebibyte,mebibytes
data|gib|1073741824|Gibibytes|gibibyte,gibibytes
data|tib|1099511627776|Tebibytes|tebibyte,tebibytes
data|pib|1125899906842624|Pebibytes|pebibyte,pebibytes
data|kbit|125|Kilobits|kilobit,kilobits
data|mbit|125000|Megabits|megabit,megabits
data|gbit|125000000|Gigabits|gigabit,gigabits
data|tbit|125000000000|Terabits|terabit,terabits
angle|rad|1|Radians|radian,radians
angle|deg|0.017453292519943295|Degrees|degree,degrees,°
angle|grad|0.015707963267948967|Gradians|gon,gradian,gradians
angle|turn|6.283185307179586|Turns|turns,rev,revolution,revolutions
angle|arcmin|0.0002908882086657216|Arcminutes|arcminute,arcminutes
angle|arcsec|4.84813681109536e-6|Arcseconds|arcsecond,arcseconds
angle|mrad|0.001|Milliradians|milliradian
force|n|1|Newtons|newton,newtons
force|kn_|1000|Kilonewtons|kilonewton,kilonewtons
force|lbf|4.4482216152605|Pound-force|poundforce
force|dyn|1e-5|Dynes|dyne,dynes
force|kgf|9.80665|Kilogram-force|kilogramforce
pressure|pa|1|Pascals|pascal,pascals
pressure|kpa|1000|Kilopascals|kilopascal,kilopascals
pressure|mpa|1e6|Megapascals|megapascal,megapascals
pressure|hpa|100|Hectopascals|hectopascal,hectopascals
pressure|bar|100000|Bar|bars
pressure|mbar|100|Millibar|millibar,millibars
pressure|atm|101325|Atmospheres|atmosphere,atmospheres
pressure|psi|6894.757293168|PSI|
pressure|torr|133.322368421|Torr|
pressure|mmhg|133.322387415|mmHg|
pressure|inhg|3386.389|inHg|
energy|j|1|Joules|joule,joules
energy|kj|1000|Kilojoules|kilojoule,kilojoules
energy|megajoule|1e6|Megajoules|megajoules
energy|cal|4.184|Calories|calorie,calories
energy|kcal|4184|Kilocalories|kilocalorie,kilocalories,kcals
energy|wh|3600|Watt-hours|watthour,watthours
energy|kwh|3.6e6|Kilowatt-hours|kilowatthour,kilowatthours
energy|mwh|3.6e9|Megawatt-hours|megawatthour
energy|btu|1055.05585262|BTU|btus
energy|ev|1.602176634e-19|Electronvolts|electronvolt
energy|erg|1e-7|Ergs|ergs
energy|therm|105505585.262|Therms|therms
power|w|1|Watts|watt,watts
power|kw|1000|Kilowatts|kilowatt,kilowatts
power|megawatt|1e6|Megawatts|megawatts
power|gw|1e9|Gigawatts|gigawatt,gigawatts
power|milliwatt|0.001|Milliwatts|milliwatts
power|hp|745.69987158227|Horsepower|horsepower
power|ps|735.49875|Metric horsepower|metrichorsepower
freq|hz|1|Hertz|hertz
freq|khz|1e3|Kilohertz|kilohertz
freq|mhz|1e6|Megahertz|megahertz
freq|ghz|1e9|Gigahertz|gigahertz
freq|thz|1e12|Terahertz|terahertz
freq|rpm|1/60|RPM|
current|amp|1|Amperes|amps,ampere,amperes
current|ma|0.001|Milliamperes|milliamp,milliamps,milliampere
current|ka|1000|Kiloamperes|kiloamp
voltage|v|1|Volts|volt,volts
voltage|mv|0.001|Millivolts|millivolt,millivolts
voltage|kv|1000|Kilovolts|kilovolt,kilovolts
resistance|ohm|1|Ohms|ohms,ω
resistance|kohm|1000|Kilohms|kilohm,kilohms
charge|coulomb|1|Coulombs|coulombs
charge|mah|3.6|Milliamp-hours|milliamphour
charge|ah|3600|Amp-hours|amphour
";

lazy_static::lazy_static! {
    static ref UNITS: (Vec<Unit>, HashMap<String, usize>) = build();
}

fn eval_factor(s: &str) -> f64 {
    let mut acc = 1.0;
    let mut op = '*';
    let mut cur = String::new();
    let flush = |acc: &mut f64, op: char, cur: &str| {
        let v: f64 = cur.parse().unwrap_or(1.0);
        if op == '*' { *acc *= v } else { *acc /= v }
    };
    for c in s.chars() {
        if c == '*' || c == '/' {
            flush(&mut acc, op, &cur);
            cur.clear();
            op = c;
        } else {
            cur.push(c);
        }
    }
    flush(&mut acc, op, &cur);
    acc
}

fn expansion(sym: &str) -> Option<&'static str> {
    match sym {
        "mph" => Some("mi/h"),
        "kph" => Some("km/h"),
        "fps" => Some("ft/s"),
        _ => None,
    }
}

fn build() -> (Vec<Unit>, HashMap<String, usize>) {
    let mut v = Vec::new();
    let mut m = HashMap::new();
    for line in TABLE.lines() {
        let p: Vec<&str> = line.split('|').collect();
        if p.len() < 5 {
            continue;
        }
        let sym = match p[1] {
            "kn_" => "kN",
            "c" => "°C",
            "f" => "°F",
            "k" => "K",
            o => o,
        }
        .to_string();
        let sym = match sym.strip_suffix('2').or(sym.strip_suffix('3')) {
            Some(b) if !b.is_empty() && b.chars().all(|c| c.is_alphabetic()) => {
                format!("{b}{}", if sym.ends_with('2') { '²' } else { '³' })
            }
            _ => sym,
        };
        let u = Unit {
            sym: sym.clone(),
            long: p[3].to_string(),
            dim: dim_of(p[0]),
            f: eval_factor(p[2]),
            off: p.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            expand: expansion(p[1]),
        };
        let idx = v.len();
        v.push(u);
        m.entry(p[1].to_string()).or_insert(idx);
        for a in p[4].split(',').filter(|a| !a.is_empty()) {
            m.entry(a.to_string()).or_insert(idx);
        }
    }
    (v, m)
}

pub fn unit_count() -> usize {
    UNITS.0.len()
}

pub fn name_count() -> usize {
    UNITS.1.len()
}

fn find(name: &str, cur: &CurrencyTable) -> Option<Unit> {
    if let Some(&i) = UNITS.1.get(name) {
        return Some(UNITS.0[i].clone());
    }
    cur.unit(name)
}

/// Look a unit up by (lowercase) name, tolerating plural `s`/`es`.
pub fn lookup(name: &str, cur: &CurrencyTable) -> Option<Unit> {
    if let Some(u) = find(name, cur) {
        return Some(u);
    }
    if name.len() > 3 {
        if let Some(b) = name.strip_suffix("es") {
            if let Some(u) = find(b, cur) {
                return Some(u);
            }
        }
        if let Some(b) = name.strip_suffix('s') {
            return find(b, cur);
        }
    }
    None
}

/// Format a dimension as base-unit text when no display unit survives.
pub fn dim_text(d: &Dim) -> String {
    const N: [&str; 8] = ["m", "kg", "s", "K", "B", "rad", "USD", "A"];
    let mut s = Vec::new();
    for (i, e) in d.iter().enumerate() {
        match *e {
            0 => {}
            1 => s.push(N[i].to_string()),
            e => s.push(format!("{}^{}", N[i], e)),
        }
    }
    s.join("·")
}
