/// 通货元数据
struct CurrencyMeta {
    code: &'static str,
    zh_name: &'static str,
    #[allow(dead_code)]
    short_name: &'static str,
}

/// 通货中文名称映射表
static CURRENCIES: &[CurrencyMeta] = &[
    CurrencyMeta {
        code: "chaos",
        zh_name: "混沌石",
        short_name: "混沌",
    },
    CurrencyMeta {
        code: "exalted",
        zh_name: "崇高石",
        short_name: "崇高",
    },
    CurrencyMeta {
        code: "divine",
        zh_name: "神圣石",
        short_name: "神圣",
    },
    CurrencyMeta {
        code: "regal",
        zh_name: "富豪石",
        short_name: "富豪",
    },
    CurrencyMeta {
        code: "alchemy",
        zh_name: "点金石",
        short_name: "点金",
    },
    CurrencyMeta {
        code: "vaal",
        zh_name: "瓦尔宝珠",
        short_name: "瓦尔",
    },
    CurrencyMeta {
        code: "mirror",
        zh_name: "卡兰德魔镜",
        short_name: "魔镜",
    },
    CurrencyMeta {
        code: "annulment",
        zh_name: "剥离石",
        short_name: "剥离",
    },
    CurrencyMeta {
        code: "blessed",
        zh_name: "祝福石",
        short_name: "祝福",
    },
    CurrencyMeta {
        code: "cartographer",
        zh_name: "制图钉",
        short_name: "制图",
    },
    CurrencyMeta {
        code: "chromatic",
        zh_name: "五彩石",
        short_name: "五彩",
    },
    CurrencyMeta {
        code: "engineer",
        zh_name: "工程师的宝珠",
        short_name: "工程师",
    },
    CurrencyMeta {
        code: "gemcutter",
        zh_name: "宝石匠的棱镜",
        short_name: "宝石棱镜",
    },
    CurrencyMeta {
        code: "jeweller",
        zh_name: "珠宝匠的宝珠",
        short_name: "珠宝匠",
    },
    CurrencyMeta {
        code: "fusing",
        zh_name: "链接石",
        short_name: "链接",
    },
    CurrencyMeta {
        code: "scouring",
        zh_name: "重铸石",
        short_name: "重铸",
    },
    CurrencyMeta {
        code: "alteration",
        zh_name: "改造石",
        short_name: "改造",
    },
    CurrencyMeta {
        code: "transmutation",
        zh_name: "蜕变石",
        short_name: "蜕变",
    },
    CurrencyMeta {
        code: "augmentation",
        zh_name: "增幅石",
        short_name: "增幅",
    },
    CurrencyMeta {
        code: "chance",
        zh_name: "机会石",
        short_name: "机会",
    },
    CurrencyMeta {
        code: "glassblower",
        zh_name: "玻璃弹珠",
        short_name: "弹珠",
    },
    CurrencyMeta {
        code: "armourer",
        zh_name: "护甲片",
        short_name: "护甲",
    },
    CurrencyMeta {
        code: "blacksmith",
        zh_name: "磨刀石",
        short_name: "磨刀",
    },
    CurrencyMeta {
        code: "silver",
        zh_name: "银币",
        short_name: "银币",
    },
];

/// 根据通货代码获取中文名称
pub fn currency_zh_name(code: &str) -> &str {
    if let Some(meta) = CURRENCIES
        .iter()
        .find(|c| c.code.eq_ignore_ascii_case(code))
    {
        meta.zh_name
    } else {
        code // 未知通货回退到原始代码
    }
}

/// 格式化价格：1.5 chaos → "1.5 混沌石"
pub fn format_price_zh(amount: f64, currency: &str) -> String {
    if amount == (amount as i32) as f64 {
        format!("{} {}", amount as i32, currency_zh_name(currency))
    } else {
        format!("{:.1} {}", amount, currency_zh_name(currency))
    }
}

/// 格式化价格字符串（兼容旧接口）
#[allow(dead_code)]
pub fn format_price_str_zh(amount: Option<f64>, currency: Option<&str>) -> String {
    match (amount, currency) {
        (Some(amt), Some(curr)) => format_price_zh(amt, curr),
        (Some(amt), None) => format!("{:.1}", amt),
        _ => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_currency_is_localized() {
        assert_eq!(currency_zh_name("chaos"), "混沌石");
        assert_eq!(currency_zh_name("exalted"), "崇高石");
        assert_eq!(currency_zh_name("divine"), "神圣石");
        assert_eq!(currency_zh_name("mirror"), "卡兰德魔镜");
    }

    #[test]
    fn unknown_currency_falls_back_to_code() {
        assert_eq!(currency_zh_name("unknown_gem"), "unknown_gem");
    }

    #[test]
    fn case_insensitive_lookup() {
        assert_eq!(currency_zh_name("Chaos"), "混沌石");
        assert_eq!(currency_zh_name("EXALTED"), "崇高石");
    }

    #[test]
    fn format_price_zh_integer() {
        assert_eq!(format_price_zh(10.0, "chaos"), "10 混沌石");
    }

    #[test]
    fn format_price_zh_fractional() {
        assert_eq!(format_price_zh(1.5, "divine"), "1.5 神圣石");
    }
}
