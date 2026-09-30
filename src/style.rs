//! 用于将CSS样式应用于DOM的代码。
//!
//! 目前这还不太有趣。后面会变得更有意思
//! 如果我添加对复合选择器的支持，就会变得很复杂。

use crate::dom::{Node, NodeType, ElementData};
use crate::css::{Stylesheet, Rule, Selector, SimpleSelector, Value, Specificity};
use std::collections::HashMap;

/// 将CSS属性名称映射到值。
pub type PropertyMap = HashMap<String, Value>;

/// 具有关联样式数据的节点。
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

#[derive(PartialEq)]
pub enum Display {
    Inline,
    Block,
    None,
}

impl<'a> StyledNode<'a> {
    /// 如果存在，则返回属性的指定值，否则返回“None”。
    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()    // get返回Option<&Value> 通过cloned转换为Option<Value>
    }

    /// 返回属性“name”的指定值，否则返回属性“fallback_name”
    /// 如果两者都不存在，则设置为“默认”。
    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.value(name).unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }

    /// `display`属性的值（默认为内联）。
    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match &*s {  // String -> *String = str -> &*String=&str
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }
}

/// 将样式表应用于整个DOM树，返回StyledNode树。
///
/// 这只会查找当前指定的值。最终，它也应该扩展到查找计算值，包括继承值。
pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    StyledNode {
        node: root,
        specified_values: match root.node_type {
            NodeType::Element(ref elem) => specified_values(elem, stylesheet),
            NodeType::Text(_) => HashMap::new()
        },
        children: root.children.iter().map(|child| style_tree(child, stylesheet)).collect(),
    }
}

/// 将样式应用于单个元素，返回指定的样式。
///
/// 要做的：允许多个UA/作者/用户样式表，并实现级联。
fn specified_values(elem: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    // 按照从最低到最高的具体程度来浏览规则。
    rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
    for (_, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    values
}


/// 一个CSS规则及其最具体的匹配选择器的特殊性。
type MatchedRule<'a> = (Specificity, &'a Rule);

/// 查找与给定元素匹配的所有CSS规则。
fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    //目前，我们只是对所有规则进行线性扫描。对于大型
    //文档，将规则存储在哈希表中会更有效
    //基于标签名、id、类等。
    stylesheet.rules.iter().filter_map(|rule| match_rule(elem, rule)).collect()
}

/// 如果`rule`匹配`elem`，则返回`MatchedRule`。否则返回“无”。
fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    // 找到第一个匹配的（最具体的）选择器。
    rule.selectors
        .iter().find(|selector| matches(elem, selector))
        .map(|selector| (selector.specificity(), rule))
}

/// 选择器匹配
fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match selector {
        Selector::Simple(s) => matches_simple(elem, s),
    }
}

fn matching_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    // 检查标签类型选择器
    // 将css代码中的标签和dom中的标签进行比较
    if selector.tag_name.iter().any(|name| elem.tag_name != *name) {
        return false;
    }

    // 检查ID选择器
    if selector.id.iter().any(|id| elem.id() != Some(id)) {
        return false;
    }

    // 检查类选择器
    if selector.class.iter().any(|class| !elem.classes().contains(class.as_str())) {
        return false;
    }

    // 我们没有发现任何不匹配的选择器组件。
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{Declaration, Rule, Selector, SimpleSelector, Stylesheet, Unit, Value};
    use crate::dom::{elem, text, AttrMap};

    /// 从便捷的 `(名称, 值)` 对构建属性映射。
    fn attrs(pairs: &[(&str, &str)]) -> AttrMap {
        pairs.iter().map(|(name, value)| ((*name).to_string(), (*value).to_string())).collect()
    }

    /// 从可选标记名、可选id和类列表构建一个简单的选择器。
    fn selector(tag: Option<&str>, id: Option<&str>, classes: &[&str]) -> Selector {
        Selector::Simple(SimpleSelector {
            tag_name: tag.map(String::from),
            id: id.map(String::from),
            class: classes.iter().map(|class| (*class).to_string()).collect(),
        })
    }

    /// 从选择器和“（property，value）”声明对构建规则。
    fn rule(selectors: Vec<Selector>, declarations: Vec<(&str, Value)>) -> Rule {
        Rule {
            selectors,
            declarations: declarations
                .into_iter()
                .map(|name, value| Declaration {name: name.to_string(), value})
                .collect(),
        }
    }

    /// 根据规则构建样式表。
    fn stylesheet(rules: Vec<Rule>) -> Stylesheet {
        Stylesheet { rules }
    }

    /// 根据样式表设置元素节点的样式，并返回已设置样式的根。
    fn style_element<'a>(node: &'a Node, css: &'a Stylesheet) -> StyledNode<'a> {
        style_tree(node, css)
    }

    /// 借用样式化节点后面的DOM节点的标记名。
    fn tag_name(node: &StyledNode) -> &'a str {
        match &node.node.node_type {
            NodeType::Element(data) => &data.tag_name,
            NodeType::Text(data) => panic!("预期为元素节点, 但实际为文本 {data:?}"),
        }
    }

    #[test]
    fn value_returns_specified_value_or_none() {
        let node = elem("p".to_string(), attrs(&[]), vec![]);
        let css = stylesheet(vec![rule(
            vec![selector(Some("p"), None, &[])],
            vec![("color", Value::Keyword("red".to_string()))],
        )]);
        let styled = style_tree(&node, &css);

        assert_eq!(styled.value("color"), Some(Value::Keyword("red".to_string())));
        assert_eq!(styled.value("margin"), None);
    }

    #[test]
    fn lookup_prefers_name_then_fallback_then_default() {
        let node = elem("p".to_string(), attrs(&[]), vec![]);
        let fallback = stylesheet(vec![rule(
            vec![selector(Some("p"), None, &[])],
            vec![("color", Value::Keyword("red".to_string()))],
        )]);
        let styled = style_tree(&node, &css);

        assert_eq!(styled.value("color"), Some(Value::Keyword("red".to_string())));
        assert_eq!(styled.value("margin"), None);
    }

    #[test]
    fn lookup_prefers_name_then_fallback_then_default() {
        let node = elem("p".to_string(), attrs(&[]), vec![]);
        let fallback = stylesheet(vec![rule(
            vec![selector(Some("p"), None, &[])],
            vec![("margin", Value::Length(3.0, Unit::Px))],
        )]);
        let both = stylesheet(vec![rule(
            vec![selector(Some("p"), None, &[])],
            vec![
                ("margin-left", Value::Length(2.0, Unit::Px)),
                ("margin", Value::Length(3.0, Unit::Px)),
            ],
        )]);
        let none = stylesheet(vec![]);
        let zero = Value::Length(0.0, Unit::Px);

        // 未指定`margin-left`，因此使用简写属性`margin`。
        assert_eq!(
            style_tree(&node, &fallback).lookup("margin-left", "margin", &zero),
            Value::Length(3.0, Unit::Px)
        );
        // 当两者均有规定时，长写法优于缩写法。
        assert_eq!(
            style_tree(&node, &both).lookup("margin-left", "margin", &zero),
            Value::Length(2.0, Unit::Px)
        );
        // 未指定任一属性，因此采用默认值。
        assert_eq!(style_tree(&node, &none).lookup("margin-left", "margin", &zero), zero);
    }

    #[test]
    fn display_maps_keywords_and_defaults_to_inline() {
        let node = elem("p".to_string(), attrs(&[]), vec![]);
        let css_with = |value: Value| {
            stylesheet(vec![rule(
                vec![selector(Some("p"), None, &[])],
                vec![("display", value)],
            )])
        };

        assert!(style_tree(&node, &css_with(Value::Keyword("block".to_string()))).display()
            == Display::Block);
        assert!(style_tree(&node, &css_with(Value::Keyword("none".to_string()))).display()
            == Display::None);
        assert!(style_tree(&node, &css_with(Value::Keyword("inline".to_string()))).display()
            == Display::Inline);
        // 未知的关键字和缺失的声明会退回内联。
        assert!(style_tree(&node, &css_with(Value::Keyword("flex".to_string()))).display()
            == Display::Inline);
        assert!(style_tree(&node, &stylesheet(vec![])).display() == Display::Inline);
        // A non-keyword value is not a valid `display` keyword, so inline is used.
        assert!(style_tree(&node, &css_with(Value::Length(1.0, Unit::Px))).display()
            == Display::Inline);
    }

    #[test]
    fn style_tree_gives_text_nodes_no_specified_values() {
        let node = text("hello".to_string());
        let styled = style_tree(&node, &stylesheet(vec![]));

        assert!(styled.specified_values.is_empty());
        assert!(styled.children.is_empty());
        assert_eq!(styled.value("color"), None);
    }

    #[test]
    fn style_tree_preserves_tree_structure_and_styles_each_node() {
        let child = elem("span".to_string(), attrs(&[]), vec![]);
        let node = elem("div".to_string(), attrs(&[("id", "main")]), vec![child]);
        let css = stylesheet(vec![
            rule(
                vec![selector(Some("div"), None, &[])],
                vec![("display", Value::Keyword("block".to_string()))],
            ),
            rule(
                vec![selector(Some("span"), None, &[])],
                vec![("display", Value::Keyword("none".to_string()))],
            ),
        ]);
        let styled = style_tree(&node, &css);

        assert_eq!(tag_name(&styled), "div");
        assert_eq!(styled.children.len(), 1);
        assert_eq!(tag_name(&styled.children[0]), "span");
        assert!(styled.display() == Display::Block);
        assert!(styled.children[0].display() == Display::None);
    }

    #[test]
    fn higher_specificity_rule_wins() {
        let node = elem("p".to_string(), attrs(&[("id", "main"), ("class", "note")]), vec![]);
        let css = stylesheet(vec![
            rule(
                vec![selector(Some("p"), None, &[])],
                vec![("color", Value::Keyword("red".to_string()))],
            ),
            rule(
                vec![selector(None, None, &["note"])],
                vec![("color", Value::Keyword("green".to_string()))],
            ),
            rule(
                vec![selector(None, Some("main"), &[])],
                vec![("color", Value::Keyword("blue".to_string()))],
            ),
        ]);
        let styled = style_tree(&node, &css);

        // (1,0,0) beats (0,1,0), which beats (0,0,1).
        assert_eq!(styled.value("color"), Some(Value::Keyword("blue".to_string())));
    }

    #[test]
    fn later_declaration_wins_inside_a_single_rule() {
        let node = elem("p".to_string(), attrs(&[]), vec![]);
        let css = stylesheet(vec![rule(
            vec![selector(Some("p"), None, &[])],
            vec![
                ("color", Value::Keyword("red".to_string())),
                ("color", Value::Keyword("blue".to_string())),
            ],
        )]);
        let styled = style_tree(&node, &css);

        assert_eq!(styled.value("color"), Some(Value::Keyword("blue".to_string())));
    }

    #[test]
    fn non_matching_rules_are_ignored() {
        let node = elem("p".to_string(), attrs(&[]), vec![]);
        let css = stylesheet(vec![rule(
            vec![selector(Some("span"), None, &[])],
            vec![("color", Value::Keyword("red".to_string()))],
        )]);
        let styled = style_tree(&node, &css);

        assert!(styled.specified_values.is_empty());
    }

    #[test]
    fn matches_simple_selector_checks_tag_id_and_class() {
        let data = ElementData {
            tag_name: "p".to_string(),
            attrs: attrs(&[("id", "main"), ("class", "note important")]),
        };

        // 通用选择器没有组件，所以它总是匹配的。
        assert!(matches_simple_selector(&data, &SimpleSelector {
            tag_name: None,
            id: None,
            class: vec![],
        }));
        assert!(matches_simple_selector(
            &data,
            &SimpleSelector { tag_name: Some("p".to_string()), id: None, class: vec![] }
        ));
        assert!(matches_simple_selector(
            &data,
            &SimpleSelector { tag_name: None, id: Some("main".to_string()), class: vec![] }
        ));
        assert!(matches_simple_selector(
            &data,
            &SimpleSelector {
                tag_name: Some("p".to_string()),
                id: Some("main".to_string()),
                class: vec!["note".to_string(), "important".to_string()],
            }
        ));

        // 任何单个不匹配的组件都会拒绝该元素。
        assert!(!matches_simple_selector(
            &data,
            &SimpleSelector { tag_name: Some("div".to_string()), id: None, class: vec![] }
        ));
        assert!(!matches_simple_selector(
            &data,
            &SimpleSelector { tag_name: None, id: Some("other".to_string()), class: vec![] }
        ));
        assert!(!matches_simple_selector(
            &data,
            &SimpleSelector { tag_name: None, id: None, class: vec!["missing".to_string()] }
        ));
        // 类在空格分隔的标记上匹配，而不是在子字符串上匹配。
        assert!(!matches_simple_selector(
            &data,
            &SimpleSelector { tag_name: None, id: None, class: vec!["impo".to_string()] }
        ));
    }

    #[test]
    fn match_rule_uses_the_first_matching_selector() {
        let data = ElementData {
            tag_name: "div".to_string(),
            attrs: attrs(&[("class", "wide")]),
        };
        let css_rule = rule(
            vec![
                selector(Some("div"), None, &["wide"]),
                selector(Some("div"), None, &[]),
            ],
            vec![("display", Value::Keyword("block".to_string()))],
        );

        // 第一个（最具体的）匹配选择器决定特异性。
        let (specificity, matched) = match_rule(&data, &css_rule).unwrap();
        assert_eq!(specificity, (0, 1, 1));
        assert_eq!(matched.declarations[0].name, "display");

        // 没有类，第二个选择器匹配的特异性较低。
        let plain = ElementData { tag_name: "div".to_string(), attrs: attrs(&[]) };
        assert_eq!(match_rule(&plain, &css_rule).unwrap().0, (0, 0, 1));

        // 不匹配的元素不会选择任何内容。
        let other = ElementData { tag_name: "span".to_string(), attrs: attrs(&[]) };
        assert!(match_rule(&other, &css_rule).is_none());
    }

    #[test]
    fn matching_rules_collects_every_matching_rule() {
        let data = ElementData {
            tag_name: "p".to_string(),
            attrs: attrs(&[("id", "main")]),
        };
        let css = stylesheet(vec![
            rule(
                vec![selector(Some("p"), None, &[])],
                vec![("margin", Value::Length(1.0, Unit::Px))],
            ),
            rule(vec![selector(Some("span"), None, &[])], vec![]),
            rule(
                vec![selector(None, Some("main"), &[])],
                vec![("padding", Value::Length(2.0, Unit::Px))],
            ),
        ]);

        let matched = matching_rules(&data, &css);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].0, (0, 0, 1));
        assert_eq!(matched[1].0, (1, 0, 0));
    }
}
