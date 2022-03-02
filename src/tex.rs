/// Parse a TeX pattern file, calling `f` with each pattern.
pub fn parse<F>(tex: &str, mut f: F)
where
    F: FnMut(&str),
{
    let mut s = Scanner(tex);
    while let Some(c) = s.eat() {
        match c {
            '%' => {
                s.eat_while(|c| c != '\n');
            }
            '\\' if s.eat_if("patterns{") => loop {
                let pat = s.eat_while(|c| c != '}' && c != '%' && !c.is_whitespace());
                if !pat.is_empty() {
                    f(pat);
                }
                match s.eat() {
                    Some('}') => break,
                    Some('%') => s.eat_while(|c| c != '\n'),
                    _ => s.eat_while(char::is_whitespace),
                };
            },
            _ => {}
        }
    }
}

struct Scanner<'a>(&'a str);

impl<'a> Scanner<'a> {
    fn eat(&mut self) -> Option<char> {
        let mut chars = self.0.chars();
        let c = chars.next();
        self.0 = chars.as_str();
        c
    }

    fn eat_if(&mut self, pat: &str) -> bool {
        let matches = self.0.starts_with(pat);
        if matches {
            self.0 = &self.0[pat.len() ..];
        }
        matches
    }

    fn eat_while(&mut self, f: fn(char) -> bool) -> &'a str {
        let mut offset = 0;
        let mut chars = self.0.chars();
        while chars.next().map_or(false, f) {
            offset = self.0.len() - chars.as_str().len();
        }
        let head = &self.0[.. offset];
        self.0 = &self.0[offset ..];
        head
    }
}
