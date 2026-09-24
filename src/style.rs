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
