use super::Template;
use crate::libs::types::Content;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn get() -> Template {
    Template {
        dependencies: dependencies(),
        dev_dependencies: dev_dependencies(),
        tsconfig: tsconfig(),
        eslintrc: eslintrc(),
        app: app(),
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

fn app() -> HashMap<String, Content> {
    HashMap::from([("src/components/About.tsx".into(), Content::Str(component_page("About".into())))])
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
