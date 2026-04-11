use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub enum Step {
    Single(Action),
    Parallel(Vec<Action>),
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub params: HashMap<String, String>,
    pub repeat: u32,
}

impl FromStr for Action {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // "AddCheese(amount=2)" ou "MakeDough" ou "AddCheese(amount=2)^3"
        let (s, repeat) = match s.split_once('^') {
            Some((left, n)) => (left, n.parse::<u32>().map_err(|e| e.to_string())?),
            None => (s, 1),
        };

        let (name, params) = match s.split_once('(') {
            Some((name, rest)) => {
                let raw = rest.trim_end_matches(')');
                let params = raw
                    .split(',')
                    .filter_map(|kv| kv.split_once('='))
                    .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                    .collect();
                (name.trim().to_string(), params)
            }
            None => (s.trim().to_string(), HashMap::new()),
        };

        Ok(Action { name, params, repeat })
    }
}

impl FromStr for Recipe {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // "Margherita =\n    MakeDough\n    -> ..."
        let (name, rest) = s.split_once('=').ok_or("pas de '='")?;
        let name = name.trim().to_string();

        let steps = rest
            .split("->")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.starts_with('[') {
                    // "[AddCheese(amount=2), AddBasil(leaves=3)]"
                    let inner = s.trim_start_matches('[').trim_end_matches(']');
                    let actions = inner
                        .split(',')
                        .map(|a| a.trim().parse::<Action>())
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Step::Parallel(actions))
                } else {
                    Ok(Step::Single(s.parse::<Action>()?))
                }
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(Recipe { name, steps })
    }
}

/// Charge toutes les recettes depuis un fichier.
pub fn load_recipes(path: &str) -> Result<HashMap<String, Recipe>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    content
        .split("\n\n")
        .filter(|s| !s.trim().is_empty())
        .map(|block| {
            let recipe = block.trim().parse::<Recipe>()?;
            Ok((recipe.name.clone(), recipe))
        })
        .collect()
}

fn format_action(a: &Action) -> String {
    let base = if a.params.is_empty() {
        a.name.clone()
    } else {
        let inner = a
            .params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}({inner})", a.name)
    };
    if a.repeat > 1 {
        format!("{base}^{}", a.repeat)
    } else {
        base
    }
}

fn format_step(step: &Step) -> String {
    match step {
        Step::Single(a) => format_action(a),
        Step::Parallel(actions) => {
            let inner = actions.iter().map(format_action).collect::<Vec<_>>().join(", ");
            format!("[{inner}]")
        }
    }
}

/// Représentation texte du fichier recette (pour `get_recipe`).
pub fn recipe_to_formula(r: &Recipe) -> String {
    let mut s = format!("{} =\n", r.name);
    for (i, step) in r.steps.iter().enumerate() {
        if i > 0 {
            s.push_str("    -> ");
        } else {
            s.push_str("    ");
        }
        s.push_str(&format_step(step));
        s.push('\n');
    }
    s
}

/// Actions dans l’ordre d’exécution (répétitions incluses).
pub fn flatten_actions(r: &Recipe) -> Vec<Action> {
    let mut out = Vec::new();
    for step in &r.steps {
        match step {
            Step::Single(a) => {
                for _ in 0..a.repeat {
                    out.push(a.clone());
                }
            }
            Step::Parallel(v) => {
                for a in v {
                    for _ in 0..a.repeat {
                        out.push(a.clone());
                    }
                }
            }
        }
    }
    out
}