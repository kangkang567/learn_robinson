//! 一个用于解析CSS极小子集的简单解析器。
//!
//! 为了支持更多CSS语法，最简单的方法可能就是替换这个
//! 手写解析器与基于库或解析器生成器的解析器。

// Data structures:

#[derive(Debug)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

#[derive(Debug)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
}

#[derive(Debug, Clone, PartialEq, Default, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let Selector::Simple(ref simple) = *self;
        let a = simple.id.iter().count();
        let b = simple.class.len();
        let c = simple.tag_name.iter().count();
        (a, b, c)
    }
}

impl Value {
    /// Return the size of a length in px, or zero for non-lengths.
    pub fn to_px(&self) -> f32 {
        match *self {
            Value::Length(f, Unit::Px) => f,    // 通过匹配模型将f值给拿出来
            _ => 0.0    // 默认为0
        }
    }
}

/// 解析整个CSS样式表。
pub fn parse(source: String) -> Stylesheet {
    let mut parser = Parser { pos: 0, input: source };
    Stylesheet { rules: parser.parse_rules() }
}

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    /// 解析由可选空格分隔的规则集列表。
    fn parse_rules(&mut self) -> Vec<Rule> {
        let mut rules = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() { break }
            rules.push(self.parse_rule());
        }
        rules
    }

    /// 解析一个规则集：`<选择器>{<声明>}`。
    fn parse_rule(&mut self) -> Rule {
        Rule {
            selectors: self.parse_selectors(),
            declarations: self.parse_declarations(),
        }
    }

    /// 解析以逗号分隔的选择器列表。
    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => { self.consume_char(); self.consume_whitespace(); }
                '{' => break,
                c => panic!("选择器列表中有意外字符 {}", c)
            }
        }
        // 首先返回具有最高特异性的选择器，用于匹配。
        selectors.sort_by_key(|s| s.specificity());
        selectors
    }

    /// 解析一个简单的选择器，例如：`type#id.class1.class2.class3`
    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut selector = SimpleSelector { tag_name: None, id: None, class: Vec::new() };
        while !self.eof() {
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier());
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    // 通用选择器
                    self.consume_char();
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => break
            }
        }
        selector
    }

    /// 解析包含在“｛…｝”中的声明列表。
    fn parse_declarations(&mut self) -> Vec<Declaration> {
        self.expect_char('{');
        let mut declarations = Vec::new();
        loop {
            self.consume_whitespace();
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            declarations.push(self.parse_declaration());
        }
        declarations
    }

    /// 解析一个<property>：<value>；声明。
    fn parse_declaration(&mut self) -> Declaration {
        let name = self.parse_identifier();
        self.consume_whitespace();
        self.expect_char(':');
        self.consume_whitespace();
        let value = self.parse_value();
        self.consume_whitespace();
        self.expect_char(';');

        Declaration { name, value }
    }

    //  解析值的方法:

    fn parse_value(&mut self) -> Value {
        match self.next_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            _ => Value::Keyword(self.parse_identifier())
        }
    }

    fn parse_length(&mut self) -> Value {
        Value::Length(self.parse_float(), self.parse_unit())
    }

    fn parse_float(&mut self) -> f32 {
        self.consume_while(|c| matches!(c, '0'..='9' | '.')).parse::<f32>().unwrap()
    }

    fn parse_unit(&mut self) -> Unit {
        match &self.parse_identifier().to_ascii_lowercase() {
            "px" => Unit::Px,
            _ => panic!("unrecognized unit")
        }
    }

    /// 解析处理 #ffffff 表示的颜色
    fn parse_color(&mut self) -> Value {
        self.expect_char('#');
        Value::ColorValue(Color {
            r: self.parse_hex_pair(),
            g: self.parse_hex_pair(),
            b: self.parse_hex_pair(),
            a: 255
        })
    }

    /// 解析两个十六进制数字。
    fn parse_hex_pair(&mut self) -> u8 {
        let s = &self.input[self.pos .. self.pos + 2]; // 获取两个字节的数据, 正常的 a~z A~Z 都是一个字节存储
        self.pos += 2;
        u8::from_str_radix(s, 16).unwrap()
    }

    /// 解析属性名称或关键字。
    fn parse_identifier(&mut self) -> String {
        self.consume_while(valid_identifier_char)
    }

    /// 消费并丢弃零个或多个空白字符。
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// 持续消费字符，直到`test`返回false。
    fn consume_while(&mut self, test: impl Fn(char) -> bool) -> String {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }
        result
    }
    // 上方的 impl Fn(char) -> bool 是函数特征约束, impl是一个语法糖
    // fn consume_while<T>(&mut self, test: T) -> String
    // where
    //     T: Fn(char) -> bool
    // {
    //     let mut result = String::new();
    //     while !self.eof() && test(self.next_char()) {
    //         result.push(self.consume_char());
    //     }
    //     result
    // }
    

    /// 返回当前字符，并将self.pos前进到下一个字符。
    fn consume_char(&mut self) -> char {
        let c = self.next_char();
        self.pos += c.len_utf8();
        c
    }

    ///如果在当前位置找到确切的字符'c'，则消耗它。
    ///否则，报错。
    fn expect_char(&mut self, c: char) {
        if self.consume_char() != c {
            panic!("Expected {:?} at byte {} but it was not found", c, self.pos);
        }
    }


    /// 读取当前字符而不消耗它。
    fn next_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    /// 如果所有输入都被消耗，则返回true。
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

fn valid_identifier_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_')
}


/// 测试模块
#[cfg(test)]
mod tests {
    // 引入父模块的所有内容
    use super::*;

    #[test]
    fn test_valid_identifier_char() {
        assert_eq!(valid_identifier_char('a'), true);
        assert_eq!(valid_identifier_char('g'), true);
        assert_eq!(valid_identifier_char('z'), true);
        assert_eq!(valid_identifier_char('A'), true);
        assert_eq!(valid_identifier_char('O'), true);
        assert_eq!(valid_identifier_char('L'), true);
        assert_eq!(valid_identifier_char('0'), true);
        assert_eq!(valid_identifier_char('5'), true);
        assert_eq!(valid_identifier_char('9'), true);
        assert_eq!(valid_identifier_char('_'), true);
        assert_eq!(valid_identifier_char('-'), true);
    }

    let parse_data = Parser { pos: 0, input: "" }

    /// 解析样式表并返回其第一条规则。
    fn parse_first_rule(css: &str) -> Rule {
        let mut rules = parse(css.to_string()).rules;
        assert_eq!(rules.len(), 1); // 只允许有一条结果
        rules.pop().unwrap()
    }

    /// 从“选择器”中借用简单的选择器。
    fn simple_selector(selector: &Selector) -> &SimpleSelector {
        match selector {
            Selector::Simple(simple) => simple,
        }
    }

    /// 借用简单选择器的类名作为`&str`。
    fn class_names(selector: &SimpleSelector) -> Vec<&str> {
        selector.class.iter().map(String::as_str).collect::<Vec<&str>>()
    }
    
    /// 从方便的输入构建一个“Selector::Simple”，用于直接单元测试。
    fn simple(tag: Option<&str>, id: Option<&str>, classes: &[&str]) -> Selector {
        Selector::Simple(SimpleSelector {
            tag_name: tag.map(String::from),
            id: id.map(String::from),
            class: classes.iter().map(|s| s.to_string()).collect()
        })
    }

    #[test]
    fn parse_empty_stylesheet() {
        assert!(parse(String::new()).rules.is_empty());
        assert!(parse(" \n\t\r  ".to_string()).rules.is_empty());
    }

    #[test]
    fn parse_rule_with_keyword_declaration() {
        let rule = parse_first_rule("h1 { color: red; }");

        assert_eq!(rule.selectors.len(), 1);
        let selector = simple_selector(&rule.selectors[0]);
        // selector.tag_name为Option<String>, selector.tag_name.as_deref()为Option<&str>, Some<"h1">的类型为Option<&str>
        assert_eq!(selector.tag_name.as_deref(), Some("h1"));
        assert_eq!(selector.id, None);
        assert!(selector.class.is_empty());

        assert_eq!(rule.declarations.len(), 1);
        let declaration = &rule.declaration[0];
        assert_eq!(declaration.name, "color");
        assert_eq!(declaration.value, Value::Keyword("red".to_string())); // "red" &'static str类型, 通过to_string转换为String
    }

    #[test]
    fn parse_multiple_rules() {
        let stylesheet = parse("h1 { color: red; }\np { margin: 0px; }".to_string());
        assert_eq!(stylesheet.rules.len(), 2);
        assert_eq!(
            // tag_name是Option<String>类型 as_deref后变成 Option<&str>, Some("h1")为Option<&str>
            simple_selector(&stylesheet.rules[0].selectors[0]).tag_name.as_deref(),
            Some("h1")
        );
        assert_eq!(
            simple_selector(&stylesheet.rules[1].selectors[0]).tag_name.as_deref(),
            Some('p')
        );
    }

    #[test]
    fn parse_multiple_declarations_with_different_value_types() {
        let rule = parse_first_rule(
            "body { color: red; background-color: #00ff00; font-size: 14px; }",
        );

        let names: Vec<&str> = rule.declarations.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["color", "background-color", "font-size"]);

        assert_eq!(rule.declarations[0].value, Value::Keyword("red".to_string()));
        assert_eq!(
            rule.declarations[1].value,
            Value::ColorValue(Color { r: 0, g: 255, b: 0, a: 255 })
        );
        assert_eq!(rule.declarations[2].value, Value::Length(14.0, Unit::Px));
    }

    #[test]
    fn parse_selector_list_sorted_by_specificity() {
        // 解析时选择器列表按特异性升序排列: 越靠后的声明越晚被引用, 
        // 从而高特异性的选择器在匹配/级联中最终生效。
        let rule = parse_first_rule("h1, #main, .class { color: red; }");

        let selectors = &rule.selectors;
        assert_eq!(selectors.len(), 3);
        let specificities: Vec<Specificity> = selectors.iter().map(Selector::specificity).collect();
        // 输入依次为 h1 -> (0,0,1)、#main -> (1,0,0)、.class -> (0,1,0)，
        // 排序后应为 (0,0,1), (0,1,0), (1,0,0)。
        assert_eq!(specificities, vec![(0, 0, 1), (0, 1, 0), (1, 0, 0)]);
        assert_eq!(
            simple_selector(&selectors[0]).tag_name.as_deref(), // Option<&str>
            Some("h1")  // Option<&str>
        );
        assert_eq!(simple_selector(&selectors[1]).id.as_deref(), None);
        assert_eq!(simple_selector(&selectors[2]).id.as_deref(), Some("main"));
    }

    #[test]
    fn parse_simple_selector_with_id_and_classes() {
        let rule = parse_first_rule("a#home.nav.link { text-decoration: none; }");

        let selector = simple_selector(&rule.selectors[0]);
        assert_eq!(selector.tag_name.as_deref(), Some("a"));
        assert_eq!(selector.id.as_deref(), Some("home"));
        assert_eq!(class_names(selector), vec!["nav", "link"]);
        assert_eq!(rule.declarations[0].value, Value::Keyword("none".to_string()));
    }

    #[test]
    fn parse_universal_selector() {
        let rule = parse_first_rule("* { margin: 0px; }");

        let selector = simple_selector(&rule.selectors[0]);
        assert_eq!(selector.tag_name, None);
        assert_eq!(selector.id, None);
        assert!(selector.class.is_empty());
        assert_eq!(rule.declarations[0].value, Value::Length(0.0, Unit::Px));
    }

    #[test]
    fn parse_integer_length_value() {
        let rule = parse_first_rule("div { width: 100px; }");
        assert_eq!(rule.declarations[0].value, Value::Length(100.0, Unit::Px));
    }

    #[test]
    fn parse_fractional_length_value() {
        let rule = parse_first_rule("div { font-size: 1.5px; }");
        assert_eq!(rule.declarations[0].value, Value::Length(1.5, Unit::Px));
    }

    #[test]
    fn parse_color_value() {
        let rule = parse_first_rule("p { color: #336699 }");
        assert_eq!(rule.declarations[0].value, Value::ColorValue(Color {
            r: 0x33,
            g: 0x66,
            b: 0x99,
            a: 255
        }));
    }

    #[test]
    fn specificity_counts_id_class_and_type() {
        assert_eq!(simple(Some("p"), None, &[]).specificity(), (0, 0, 1));
        assert_eq!(simple(Some("p"), None, &["note"]).specificity(), (0, 1, 1));
        assert_eq!(simple(None, Some("home"), &["a", "b"]).specificity(), (1, 2, 0));
    }

    #[test]
    fn to_px_returns_length_only_for_px_values() {
        assert_eq!(Value::Length(12.5, Unit::Px).to_px(), 12.5);
        assert_eq!(Value::Length(0.0, Unit::Px).to_px(), 0.0);
        assert_eq!(Value::Keyword("auto".to_string()).to_px(), 0.0);
        assert_eq!(Value::ColorValue(Color::default()).to_px(), 0.0);
    }

}