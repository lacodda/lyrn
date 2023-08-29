use super::{ProjectProps, Template};
use crate::libs::types::Content;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn get(project: &ProjectProps) -> Template {
    Template {
        dependencies: dependencies(),
        dev_dependencies: dev_dependencies(),
        tsconfig: tsconfig(),
        eslintrc: eslintrc(),
        app: app(&project),
        ..Template::default()
    }
}

fn dependencies() -> Value {
    json!({
        "react": "^18.2.0",
        "react-dom": "^18.2.0",
        "react-router-dom": "^6.13.0",
        "styled-components": "^6.0.7"
    })
}

fn dev_dependencies() -> Value {
    json!({
        "@pmmmwh/react-refresh-webpack-plugin": "^0.5.10",
        "@types/react": "^18.2.7",
        "@types/react-dom": "^18.2.4",
        "eslint-plugin-react": "^7.32.2",
        "react-refresh": "^0.14.0",
    })
}

fn tsconfig() -> Value {
    json!({
        "compilerOptions": {
          "jsx": "react"
        }
    })
}

fn eslintrc() -> Value {
    json!({
        "extends": [
          "plugin:react/recommended"
        ],
        "rules": {
          "react/prop-types": "off"
        }
    })
}

fn app(_project: &ProjectProps) -> HashMap<String, Content> {
    HashMap::from([
        ("src/main.ts".into(), Content::Str(main())),
        ("src/bootstrap.tsx".into(), Content::Str(bootstrap())),
        ("src/components/App.tsx".into(), Content::Str(container_component("App".into()))),
        ("src/components/Home.tsx".into(), Content::Str(home_page("Home".into()))),
        ("src/components/About.tsx".into(), Content::Str(component_page("About".into()))),
        ("src/components/Info.tsx".into(), Content::Str(component_page("Info".into()))),
        ("src/images/logo.svg".into(), Content::Str(logo("React".into()))),
    ])
}

fn main() -> String {
    r###"import('./bootstrap'); 
"###
    .into()
}

fn bootstrap() -> String {
    r###"import React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import App from '@/components/App';
import '@/ui';

createRoot(document.getElementById('app')).render(
  <BrowserRouter>
    <App />
  </BrowserRouter>,
);
"###
    .into()
}

fn component_page(name: String) -> String {
    format!(
        r###"import React from 'react';
import styled from 'styled-components';

const Container = styled.div`
  display: grid;
  justify-content: center;
  align-content: center;
  background: var(--gr-lime-blue);
  width: 100%;
  height: 100%;
`;

const Title = styled.div`
  display: flex;
  h1 {{
    font-size: var(--font-size-h1);
    width: max-content;
    text-transform: uppercase;
    background: var(--teal);
    background: var(--gr-teal-blue);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }}
`;

export const {}: React.FC = () => (
  <Container>
    <Title>
      <h1>{}</h1>
    </Title>
  </Container>
);

export default {};
"###,
        name, name, name,
    )
}

fn home_page(name: String) -> String {
    format!(
        r###"import React, {{ useState }} from 'react';
import styled from 'styled-components';
import {{ Button }} from '@/ui';
import logo from '@/images/logo.svg';

const Container = styled.div`
  display: grid;
  justify-content: center;
  align-content: center;
  background: var(--gr-azure-pink);
  width: 100%;
  height: 100%;
  .counter {{
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
  }}
  .count {{
    font-size: 2rem;
    font-weight: 700;
    color: var(--white);
  }}
`;

const Title = styled.div`
  display: flex;
  column-gap: 1rem;
  h1 {{
    font-size: var(--font-size-h1);
    width: max-content;
    text-transform: uppercase;
    background: var(--teal);
    background: var(--gr-teal-blue);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }}
`;

const {}: React.FC = () => {{
  const [count, setCount] = useState(0);

  function inc (): void {{
    setCount(count + 1);
  }}
  function dec (): void {{
    setCount(count - 1);
  }}

  return (
    <Container>
      <Title>
        <img src={{logo}} width="100" />
        <h1>React Boilerplate</h1>
      </Title>
      <div className="counter">
        <Button className="size-l" onClick={{inc}}>+</Button>
        <Button className="size-l" onClick={{dec}}>−</Button>
        <div className="count">{{count}}</div>
      </div>
    </Container>
  );
}};

export default {};
"###,
        name, name
    )
}

fn container_component(name: String) -> String {
    format!(
        r###"import React from 'react';
import {{ Routes, Route, Link }} from 'react-router-dom';
import styled from 'styled-components';
import {{ Navbar }} from '@/ui';
import Home from './Home';
import About from './About';
import Info from './Info';

const Container = styled.div`
  width: 100%;
  height: 100%;
`;

const {}: React.FC = () => (
  <Container>
    <Navbar>
      <Link to="/">home</Link>
      <Link to="/info">info</Link>
      <Link to="/about">about</Link>
    </Navbar>
    <Routes>
      <Route index element={{<Home />}} />
      <Route path="about" element={{<About />}} />
      <Route path="info" element={{<Info />}} />
    </Routes>
  </Container>
);

export default {};
"###,
        name, name
    )
}

fn logo(name: String) -> String {
    format!(
        r###"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-11.5 -10.23174 23 20.46348">
  <title>{}</title>
  <circle cx="0" cy="0" r="2.05" fill="#61dafb"/>
  <g stroke="#61dafb" stroke-width="1" fill="none">
    <ellipse rx="11" ry="4.2"/>
    <ellipse rx="11" ry="4.2" transform="rotate(60)"/>
    <ellipse rx="11" ry="4.2" transform="rotate(120)"/>
  </g>
</svg>
"###,
        name
    )
}
