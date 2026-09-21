//! 基础DOM数据结构

use std::collections::{HashMap, HashSet};

pub type AttrMap = HashMap<String, String>;

#[derive(Debug)]
pub struct Node {
    // 子节点
    pub children: Vec<Node>,

    // 每种节点类型的专用数据：
    pub node_type: NodeType,
}

#[derive(Debug)]
pub enum NodeType {
    Element(ElementData),
    Text(String),
}

#[derive(Debug)]
pub struct ElementData {
    pub tag_name: String,
    pub attrs: AttrMap,
}

// 便捷的构造函数：

pub fn text(data: String) -> Node {
    Node { children: vec![], node_type: NodeType::Text(data) }
}

pub fn elem(tag_name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
    Node {
        children,
        node_type: NodeType::Element(ElementData { tag_name, attrs })
    }
}

// 元素方法

impl ElementData {
    pub fn id(&self) -> Option(&String) {
        self.attrs.get("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.attrs.get("class") {
            Some(classlist) => classlist.split(' ').collect(),
            None => HashSet::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 从方便的“（名称，值）”对构建属性映射。
    fn attrs(pairs: &[(&str, &str)]) -> AttrMap {
        pairs.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect()
    }

    /// 将节点展开到其元素数据中，对文本节点 panicking。
    fn element_data(node: &Node) -> &ElementData {
        match &node.node_type {
            NodeType::Element(data) => data,
            NodeType::Text(text) => panic!("expected an element node, found text {text:?}"),
        }
    }

    #[test]
    fn text_constructor_makes_a_childless_text_node() {
        let node = text("hello".to_string());

        assert!(node.children.is_empty());
        match node.node_type {
            NodeType::Text(data) => assert_eq!(data, "hello"),
            _ => panic!("expected a text node"),
        }
    }

    #[test]
    fn elem_constructor_stores_tag_name_attrs_and_children() {
        let child = text("hi".to_string());
        let node = elem("p".to_string(), attrs(&[("id", "main")]), vec![child]);

        assert_eq!(node.children.len(), 1);
        assert_eq!(
            match &node.children[0].node_type {
                NodeType::Text(data) => data(),
                _ => panic!("expected a text node");
            },
            "hi"
        );

        let data = element_data(&node);
        assert_eq!(data.tag_name, "p");
        assert_eq!(data.attrs.get("id").map(String::as_str), Some("main"));
    }

    #[test]
    fn id_is_none_without_an_id_attribute() {
        let data = ElementData { tag_name: "p".to_string(), attrs: attrs(&[("class", "foo")]) };

        assert_eq!(data.id, None);
    }

    #[test]
    fn id_returns_the_attribute_value() {
        let data = ElementData { tag_name: "p".to_string(), attrs: attrs(&[("id", "main")]) };

        assert_eq!(data.id().map(String::as_str), Some("main"));
    }

    #[test]
    fn classes_is_empty_without_a_class_attribute() {
        let data = ElementData { tag_name: "p".to_string(), attrs: attrs(&[("id", "main")]) };

        assert_eq!(data.classes(), HashSet::<&str>::new());
    }

    #[test]
    fn classes_splits_a_space_separated_list() {
        let data = ElementData { tag_name: "p".to_string(), attrs: attrs(&[("class", "foo bar")]) };

        assert_eq!(data.classes(), HashSet::from(["foo", "bar"]));
    }

    #[test]
    fn classes_returns_a_single_name() {
        let data = ElementData { tag_name: "p".to_string(), attrs: attrs(&[("class", "foo")]) };

        assert_eq!(data.classes(), HashSet::from(["foo"]));
    }

    //当前的实现在单个空间上拆分，因此重复的空间
    //在集合中留下一个空字符串。这个测试确定了这种行为。
    #[test]
    fn classes_keeps_empty_entries_from_repeated_spaces() {
        let data = ElementData { tag_name: "p".to_string(), attrs: attrs(&[("class", "foo bar")])};

        assert_eq!(data.classes(), HashSet::from(["foo", "", "bar"]));
    }
}