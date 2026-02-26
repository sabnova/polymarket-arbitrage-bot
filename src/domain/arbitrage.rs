pub struct ArbSelection<'a> {
    pub leg1_token: &'a str,
    pub leg1_price: f64,
    pub leg2_token: &'a str,
    pub leg2_price: f64,
    pub leg1_outcome: &'a str,
    pub leg2_outcome: &'a str,
}

pub fn select_arb_legs<'a>(
    ask_15_up: Option<f64>,
    ask_15_down: Option<f64>,
    ask_5_up: Option<f64>,
    ask_5_down: Option<f64>,
    threshold: f64,
    t15_up: &'a str,
    t15_down: &'a str,
    t5_up: &'a str,
    t5_down: &'a str,
) -> Option<ArbSelection<'a>> {
    let sum_up_down = match (ask_15_up, ask_5_down) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    };
    let sum_down_up = match (ask_15_down, ask_5_up) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    };

    if sum_up_down.map(|s| s < threshold).unwrap_or(false) {
        return Some(ArbSelection {
            leg1_token: t15_up,
            leg1_price: ask_15_up.expect("ask_15_up checked"),
            leg2_token: t5_down,
            leg2_price: ask_5_down.expect("ask_5_down checked"),
            leg1_outcome: "Up",
            leg2_outcome: "Down",
        });
    }
    if sum_down_up.map(|s| s < threshold).unwrap_or(false) {
        return Some(ArbSelection {
            leg1_token: t15_down,
            leg1_price: ask_15_down.expect("ask_15_down checked"),
            leg2_token: t5_up,
            leg2_price: ask_5_up.expect("ask_5_up checked"),
            leg1_outcome: "Down",
            leg2_outcome: "Up",
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_up_down_when_threshold_hit() {
        let sel = select_arb_legs(
            Some(0.48),
            Some(0.6),
            Some(0.7),
            Some(0.49),
            0.99,
            "t15u",
            "t15d",
            "t5u",
            "t5d",
        )
        .expect("selection");
        assert_eq!(sel.leg1_token, "t15u");
        assert_eq!(sel.leg2_token, "t5d");
    }

    #[test]
    fn returns_none_when_no_edge() {
        let sel = select_arb_legs(
            Some(0.6),
            Some(0.6),
            Some(0.5),
            Some(0.5),
            0.99,
            "t15u",
            "t15d",
            "t5u",
            "t5d",
        );
        assert!(sel.is_none());
    }
}
