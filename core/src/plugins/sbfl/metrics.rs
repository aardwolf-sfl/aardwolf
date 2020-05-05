use std::collections::HashMap;

use yaml_rust::Yaml;

use crate::plugins::PluginInitError;

const SAFE_DENOMINATOR: f32 = 0.5;

pub trait Metric {
    fn calc(&self, aep: f32, anp: f32, aef: f32, anf: f32) -> f32;
}

impl Metric for fn(f32, f32, f32, f32) -> f32 {
    fn calc(&self, aep: f32, anp: f32, aef: f32, anf: f32) -> f32 {
        self(aep, anp, aef, anf)
    }
}

struct DStar {
    star: f32,
}

impl DStar {
    pub fn new(star: f32) -> Self {
        DStar { star }
    }
}

impl Default for DStar {
    fn default() -> Self {
        DStar::new(2.0)
    }
}

impl Metric for DStar {
    fn calc(&self, aep: f32, _anp: f32, aef: f32, anf: f32) -> f32 {
        aef.powf(self.star) / (anf + aep + SAFE_DENOMINATOR)
    }
}

fn jaccard(aep: f32, _anp: f32, aef: f32, anf: f32) -> f32 {
    aef / (aef + anf + aep + SAFE_DENOMINATOR)
}

fn op(aep: f32, anp: f32, aef: f32, _anf: f32) -> f32 {
    aef - aep / (aep + anp + SAFE_DENOMINATOR)
}

fn ochiai(aep: f32, _anp: f32, aef: f32, anf: f32) -> f32 {
    aef / (((aef + anf) * (aef + aep)).sqrt() + SAFE_DENOMINATOR)
}

fn overlap(aep: f32, _anp: f32, aef: f32, anf: f32) -> f32 {
    let min = [aef, anf, aep]
        .iter()
        .copied()
        .fold(std::f32::MAX, |min, val| if val < min { val } else { min });

    aef / (min + SAFE_DENOMINATOR)
}

fn tarantula(aep: f32, anp: f32, aef: f32, anf: f32) -> f32 {
    let expr1 = aef / (aef + anf + SAFE_DENOMINATOR);
    let expr2 = aep / (aep + anp + SAFE_DENOMINATOR);
    expr1 / (expr1 + expr2 + SAFE_DENOMINATOR)
}

fn wong1(_aep: f32, _anp: f32, aef: f32, _anf: f32) -> f32 {
    aef
}

fn zoltar(aep: f32, _anp: f32, aef: f32, anf: f32) -> f32 {
    aef / (aef + anf + aep + (10_000.0 * anf * aep / (aef + SAFE_DENOMINATOR))) + SAFE_DENOMINATOR
}

pub fn from_opts(opts: &HashMap<String, Yaml>) -> Result<Box<dyn Metric>, PluginInitError> {
    macro_rules! wrap {
        ($metric_func:ident) => {
            wrap!($metric_func as fn(f32, f32, f32, f32) -> f32)
        };
        ($metric:expr) => {
            Ok(Box::new($metric))
        };
    }

    match opts.get("metric").and_then(|metric| metric.as_str()) {
        Some("dstar") => {
            if let Some(star) = opts.get("star") {
                let star = match star {
                    Yaml::Real(real) => real.parse::<f32>().unwrap(),
                    Yaml::Integer(int) => *int as f32,
                    _ => {
                        return Err(format!(
                            "Invalid star parameter, must be an integer or real number."
                        ))
                    }
                };
                wrap!(DStar::new(star))
            } else {
                wrap!(DStar::default())
            }
        }
        Some("jaccard") => wrap!(jaccard),
        Some("op") => wrap!(op),
        Some("ochiai") => wrap!(ochiai),
        Some("overlap") => wrap!(overlap),
        Some("tarantula") => wrap!(tarantula),
        Some("wong1") => wrap!(wong1),
        Some("zoltar") => wrap!(zoltar),
        None => wrap!(DStar::default()),
        Some(unknown) => Err(format!("Unknown metric '{}'.", unknown)),
    }
}
