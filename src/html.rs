//！一个简单的HTML子集解析器。
//!
//！可以解析基本的开始标签和结束标签以及文本节点。
//!
//！尚不支持：
//!
//！*评论
//！*文件类型和处理说明
//！*自闭标签
//！*非格式良好的标记
//！*字符实体

use crate::dom;
use std::collections::HashMap;

/// 解析HTML文档并返回根元素。
pub fn parse(source: String) -> dom::Node {
    let mut nodes = Parser { pos: 0, input: source }.parse_nodes();

    // 如果文档包含根元素，只需返回它。否则，创建一个。
    if nodes.len() == 1 {
        nodes.remove(0)
    } else {
        dom::elem("html".to_string(), HashMap::new(), nodes)
    }
}

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {

    /// 解析兄弟节点序列。
    fn parse_nodes(&mut self) -> Vec<dom::Node> {
        let mut nodes = vec!();
        loop {
            self.consume_whitespace();
            if self.eof() || self.starts_with("</") {
                break;
            }
            nodes.push(self.parse_node());
        }
        nodes
    }


    /// 解析单个元素，包括其打开标记、内容和关闭标记。
    fn parse_element(&mut self) -> dom::Node {
        // 打开标签
        self.expect("<");
        let tag_name = self.parse_name();
        let attrs = self.parse_attributes();
        self.expect(">");

        // 内容
        let children = self.parse_nodes();

        // 关闭标签
        self.expect("</");
        self.expect(&tag_name);
        self.expect(">");

        dom::elem(tag_name, attrs, children)
    }

    /// 

    /// 解析单个节点
    fn parse_node(&mut self) -> dom::Node {
        if self.starts_with("<") {
            self.parse_element()    // 处理dom节点之间的嵌套
        } else {
            self.parse_text()
        }
    }

    /// 解析由空格分隔的name=“value”对列表。
    fn parse_attributes(&mut self) -> dom::AttrMap {
        let mut attributes = HashMap::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '>' {
                break;
            }
            let (name, value) = self.parse_attr();
            attributes.insert(name, value);
        }
        attributes
    }

    /// 解析单个name=“value”对。
    fn parse_attr(&mut self) -> (String, String) {
        let name = self.parse_name();
        self.expect("=");
        let value = self.parse_attr_value();
        (name, value)
    }

    /// 解析标签或属性名称。
    fn parse_name(&mut self) -> String {
        self.consume_while(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9'))
    }

    /// 解析引用的值。
    fn parse_attr_value(&mut self) -> String {
        let open_quote = self.consume_char();
        assert!(open_quote == '"' || open_quote == '\'');
        let value = self.consume_while(|c| c!= open_quote);
        let close_quote = self.consume_char();
        assert_eq!(open_quote, close_quote);
        value
    }

    /// 解析文本节点。
    fn parse_text(&mut self) -> dom::Node {
        dom::text(self.consume_while(|c| c != '<'))
    }

    /// 使用并丢弃零个或多个空白字符。
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// 持续消耗字符，直到`test`函数返回false。
    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool 
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        result
    }

    /// 返回当前字符，并将self.pos前进到下一个字符。
    fn consume_char(&mut self) -> char {
        let c = self.next_char();
        self.pos += c.len_utf8();
        c
    }

    /// 读取当前字符而不消耗它。
    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    /// 如果在当前位置找到确切的字符串's'，则使用它。
    /// 否则，恐慌。
    fn expect(&mut self, s: &str) {
        if self.starts_with(s) {
            self.pos += s.len();
        } else {
            panic!("Expected {:?} at byte {} but it was not found", s, self.pos);
        }
    }

    /// 当前输入是否以给定的字符串开头？
    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos ..].starts_with(s) // String.starts_with
    }

    /// 如果所有输入都被消耗，则返回true。
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::dom::{Node, NodeType};

    /// 解析`source`并以紧凑的S表达式形式渲染生成的树。
    fn parsed(source: &str) -> String {
        dump(&parse(source.to_string()))
    }

    /// 将节点及其子树渲染为 `(标签 属性=值 "文本")`，以便一个
    /// 测试可以通过单次比较对整个解析树进行断言。
    /// 属性已排序，因为`AttrMap`是一个`HashMap`，其迭代顺序
    /// 顺序并非确定性。
    fn dump(node: &Node) -> String {
        match &node.node_type {
            NodeType::Text(data) => format!("{data:?}"),
            NodeType::Element(data) => {
                let mut attrs: Vec<String> =
                    data.attrs.iter().map(|name, value| format!("{name}={value}")).collect();
                attrs.sort();
                let attrs = if attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", attrs.join(" "))
                };

                let children: Vec<String> = node.children.iter().map(dump).collect();
                let children = if children.is_empty() {
                    String::new()
                } else {
                    format!(" {}", children.join(" "))
                };

                format!("({}{}{})", data.tag_name, attrs, children)
            }
        }
    }

    #[test]
    fn empty_document_produces_an_empty_html_element() {
        assert_eq!(parsed(""), "(html)");
    }

    // `parse`按原样返回单个顶级节点，而不检查它
    // 是一个元素，因此纯文本文档会生成一个纯文本节点。
    #[test]
    fn a_single_top_level_node_is_returned_directly() {
        assert_eq!(parsed("Hello"), "\"Hello\"");
        assert_eq!(parsed("<p>Hello</p>"), "(p \"Hello\")");
    }

    #[test]
    fn multiple_root_elements_are_wrapped_in_an_html_element() {
        assert_eq!(parsed("<p>a</p><p>b</p>"), "(html (p \"a\") (p \"b\"))");
    }

    #[test]
    fn top_level_text_and_elements_are_wrapped_in_an_html_element() {
        assert_eq!(parsed("a<p>x</p>b"), "(html \"a\" (p \"x\") \"b\")");
    }

    #[test]
    fn nested_elements_preserve_their_structure() {
        assert_eq!(
            parsed("<div><p>one</p><span>two</span></div>"),
            "(div (p \"one\") (span \"two\"))"
        );
    }

    #[test]
    fn an_element_may_have_no_children() {
        assert_eq!(parsed("<br></br>"), "(br)");
    }

    #[test]
    fn tag_names_may_contain_digits() {
        assert_eq!(parsed("<h1>Title</h1>"), "(h1 \"Title\")");
    }

    #[test]
    fn attributes_are_parsed_with_either_quote_style() {
        assert_eq!(parsed("<a href=\"one\" title='two'>x</a>"), "(a href=one title=two \"x\")");
    }

    #[test]
    fn an_attribute_value_may_be_empty() {
        assert_eq!(parsed("<input>\"\"</input>"), "(input value=)");
    }

    #[test]
    fn whitespace_between_tags_is_discarded() {
        assert_eq!(parsed("<div>\n  <p>hi</p>\n</div>"), "(div (p \"hi\"))");
    }

    // `parse_node`跳过文本节点之前的空白，但文本节点本身会一直运行到下一个`<`，
    // 因此保留了尾随的空白。这个测试确定了这种行为的两个方面。
    #[test]
    fn leading_whitespace_before_text_is_discarded_but_trailing_whitespace_is_kept() {
        assert_eq!(parsed("<p>  a  </p>"), "(p \"a  \")");
    }

    #[test]
    fn character_entities_are_left_untouched() {
        assert_eq!(parsed("<p>a &amp; b</p>"), "(p \"a &amp; b\")");
    }

    #[test]
    #[should_panic(expected = "Expected \"</\"")]
    fn a_missing_closing_tag_panics() {
        parse("<p>hi".to_string());
    }

    #[test]
    #[should_panic(expected = "Expected \"p\"")]
    fn a_mismatched_closing_tag_panics() { parse("<p>hi</div>".to_string()); }

    #[test]
    #[should_panic]
    fn an_unquoted_attribute_value_panics() { parse("<p id=main></p>") }
}