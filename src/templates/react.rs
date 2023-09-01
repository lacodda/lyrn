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
        ("src/ui/index.ts".into(), Content::Str(ui_index())),
        ("src/ui/components/Button.tsx".into(), Content::Str(button())),
        ("src/ui/components/Navbar.tsx".into(), Content::Str(navbar())),
        ("src/ui/styles/index.scss".into(), Content::Str(style_index())),
        ("src/ui/styles/_reset.scss".into(), Content::Str(style_reset())),
        ("src/ui/styles/_variables.scss".into(), Content::Str(style_variables())),
        ("src/ui/styles/_scaffolding.scss".into(), Content::Str(style_scaffolding())),
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

fn ui_index() -> String {
    r###"import './styles/index.scss';
export * from './components/Button';
export * from './components/Navbar';
"###
    .into()
}

fn button() -> String {
    r###"import styled from 'styled-components';

export const Button = styled.button`
  background-image: linear-gradient(to top, var(--teal) 0%, var(--blue) 51%, var(--teal) 100%);
  padding: .75rem .5rem;
  text-align: center;
  text-transform: none;
  transition: 0.5s;
  background-size: auto 200%;
  color: var(--white);
  border-radius: 0.5rem;
  display: flex;
  border-color: transparent;
  font-weight: 400;
  font-size: 0.8rem;
  cursor: pointer;
  line-height: 0.5rem;
  align-items: center;
  height: min-content;
  &:hover {
    background-position: right center;
    color: var(--white);
    text-decoration: none;
  }
  &.size-l {
    text-transform: uppercase;
    font-weight: 700;
    font-size: 1.8rem;
  }
`;

export default Button;
"###
    .into()
}

fn navbar() -> String {
    r###"import React from 'react';
import styled from 'styled-components';

interface NavbarProps {
  children: React.ReactNode;
}

const NavbarMenu = styled.div`
    display: flex;
    justify-content: center;
    padding: 1rem;
    gap: 1rem;
    a {
      color: var(--white);
      font-weight: 700;
      text-decoration: none;
      transition: 1s;
      text-transform: uppercase;
      &:hover {
        color: var(--purple);
      }
    }
    input, textarea, select {
      padding: .45rem .5rem;
      border-color: transparent;
      border-radius: 0.5rem;
      font-size: 0.8rem;
      line-height: 0.5rem;
      height: min-content;
      color: var(--black);
    }
    input {
      width: 100%;
    }
  `;

const NavbarContainer = styled.div`
    position: absolute;
    width: 100%;
  `;

export const Navbar: React.FC<{ children: React.ReactNode }> = ({ children }: NavbarProps) => (
  <NavbarContainer>
    <NavbarMenu>
      {children}
    </NavbarMenu>
  </NavbarContainer>
);

export default Navbar;
"###
    .into()
}

fn style_index() -> String {
    r###"@import url('https://fonts.googleapis.com/css2?family=Noto+Sans:wght@400;700&display=swap');

@import 'reset';
@import 'variables';
@import 'scaffolding';
"###
    .into()
}

fn style_reset() -> String {
    r###"/*
  1. Use a more-intuitive box-sizing model.
*/
*,
*::before,
*::after {
  box-sizing: border-box;
}

/*
    2. Remove default margin
  */
* {
  margin: 0;
}

/*
    3. Allow percentage-based heights in the application
  */
html,
body {
  height: 100%;
}

/*
    Typographic tweaks!
    4. Add accessible line-height
    5. Improve text rendering
  */
body {
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
}

/*
    6. Improve media defaults
  */
img,
picture,
video,
canvas,
svg {
  display: block;
  max-width: 100%;
}

/*
    7. Remove built-in form typography styles
  */
input,
button,
textarea,
select {
  font: inherit;
}

/*
    8. Avoid text overflows
  */
p,
h1,
h2,
h3,
h4,
h5,
h6 {
  overflow-wrap: break-word;
}

/*
    9. Create a root stacking context
  */
#root,
#__next {
  isolation: isolate;
}
"###
    .into()
}

fn style_variables() -> String {
    r###":root {
    --black: #343434;
    --gray: #dedede;
    --blue: #06347e;
    --white: #ffffff;
    --teal: #008080;
    --azure: #31CDDD;
    --fuxia: #F700F7;
    --pink: #ec62b5;
    --violet: #9333ea;
    --purple: #38006b;
    --lime: #86efac;
    --gr-teal-blue: linear-gradient(to right, var(--teal) 0%, var(--blue) 100%);
    --gr-teal-pink: linear-gradient(to right, var(--teal) 0%, var(--pink) 100%);
    --gr-azure-fuxia: linear-gradient(to right, var(--azure) 0%, var(--fuxia) 100%);
    --gr-violet-purple: linear-gradient(to right, var(--violet) 0%, var(--purple) 100%);
    --gr-azure-pink: linear-gradient(var(--azure), var(--pink));
    --gr-lime-blue: linear-gradient(var(--lime), var(--blue));
    --gr-pink-violet: linear-gradient(var(--pink), var(--violet));
    --font-family: 'Noto Sans', sans-serif;
    --font-size: 20px;
    --font-size-h1: 2.6rem;
    --font-size-h2: 1.6rem;
    --font-size-h3: 1.1rem;
}
"###
    .into()
}

fn style_scaffolding() -> String {
    r###"html {
  font-size: var(--font-size);
  font-family: var(--font-family);
  color: var(--black);
}

#app {
  display: contents;
}
"###
    .into()
}
