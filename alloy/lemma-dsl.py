#!/usr/bin/env python3
"""
FREEHAND LEMMA DSL — Natural Language to Alloy Compiler

This module implements a Domain-Specific Language (DSL) for expressing
human-authored lemmas in near-natural notation, compiling them to Alloy
semantic propositions and lemma structures.

ARCHITECTURE:

    Natural Language / DSL
           ↓
    Lemma AST (Abstract Syntax Tree)
           ↓
    Semantic Propositions
           ↓
    Alloy Specification
           ↓
    SAT Analysis (Alloy Analyzer)
           ↓
    Counterexample / Verification Result
           ↓
    Runtime Implementation / Formal Proof
"""

import re
from dataclasses import dataclass
from typing import List, Dict, Set, Optional, Tuple
from enum import Enum

# ============================================================================
# TIER 1: LEXICAL TOKENS AND AST NODES
# ============================================================================

class TokenType(Enum):
    """Lexical token types for the DSL."""

    # Logical operators
    NOT = "¬"           # ¬ or "not"
    AND = "∧"           # ∧ or "and"
    OR = "∨"            # ∨ or "or"
    IMPLIES = "⟹"      # ⟹ or "implies" or "->"
    FORALL = "∀"        # ∀ or "forall"
    EXISTS = "∃"        # ∃ or "exists"

    # Delimiters
    LPAREN = "("
    RPAREN = ")"
    LBRACE = "{"
    RBRACE = "}"
    COMMA = ","
    DOT = "."
    COLON = ":"

    # Keywords
    LEMMA = "lemma"
    ASSUME = "assume"
    SHOW = "show"
    DEPEND_ON = "depends_on"
    WHERE = "where"

    # Atomic propositions
    ATOM = "atom"          # User-defined atomic proposition
    VARIABLE = "var"       # Variable binding

    # End of input
    EOF = "EOF"

@dataclass
class Token:
    """A lexical token."""
    type: TokenType
    value: str
    line: int
    column: int

# ============================================================================
# TIER 2: ABSTRACT SYNTAX TREE (AST)
# ============================================================================

@dataclass
class ASTNode:
    """Base class for AST nodes."""
    line: int
    column: int

@dataclass
class Atom(ASTNode):
    """Atomic proposition: a named predicate over variables."""
    name: str
    args: List[str]  # Variable names

    def __str__(self):
        if self.args:
            return f"{self.name}({', '.join(self.args)})"
        return self.name

@dataclass
class Negation(ASTNode):
    """Logical negation: ¬P"""
    operand: 'Proposition'

    def __str__(self):
        return f"¬({self.operand})"

@dataclass
class Conjunction(ASTNode):
    """Logical conjunction: P ∧ Q"""
    left: 'Proposition'
    right: 'Proposition'

    def __str__(self):
        return f"({self.left} ∧ {self.right})"

@dataclass
class Disjunction(ASTNode):
    """Logical disjunction: P ∨ Q"""
    left: 'Proposition'
    right: 'Proposition'

    def __str__(self):
        return f"({self.left} ∨ {self.right})"

@dataclass
class Implication(ASTNode):
    """Logical implication: P ⟹ Q"""
    antecedent: 'Proposition'
    consequent: 'Proposition'

    def __str__(self):
        return f"({self.antecedent} ⟹ {self.consequent})"

@dataclass
class Quantified(ASTNode):
    """Quantified proposition: ∀x P or ∃x P"""
    quantifier: str  # "forall" or "exists"
    variable: str
    body: 'Proposition'

    def __str__(self):
        symbol = "∀" if self.quantifier == "forall" else "∃"
        return f"{symbol}{self.variable} . {self.body}"

# Type alias for all proposition variants
Proposition = Atom | Negation | Conjunction | Disjunction | Implication | Quantified

@dataclass
class LemmaDefinition(ASTNode):
    """A complete lemma definition."""
    name: str
    assumptions: List[Proposition]
    conclusion: Proposition
    dependencies: Set[str]  # Names of lemmas this depends on

    def __str__(self):
        assume_str = " ∧ ".join(str(a) for a in self.assumptions)
        if assume_str:
            assume_str += " ⟹ "
        return f"Lemma {self.name}: {assume_str}{self.conclusion}"

@dataclass
class Program(ASTNode):
    """A complete program: multiple lemma definitions."""
    lemmas: List[LemmaDefinition]

    def __str__(self):
        return "\n\n".join(str(lemma) for lemma in self.lemmas)

# ============================================================================
# TIER 3: LEXER (DSL → TOKENS)
# ============================================================================

class Lexer:
    """Tokenizes DSL source code."""

    KEYWORDS = {
        "lemma": TokenType.LEMMA,
        "assume": TokenType.ASSUME,
        "show": TokenType.SHOW,
        "depends_on": TokenType.DEPEND_ON,
        "where": TokenType.WHERE,
        "not": TokenType.NOT,
        "and": TokenType.AND,
        "or": TokenType.OR,
        "implies": TokenType.IMPLIES,
        "forall": TokenType.FORALL,
        "exists": TokenType.EXISTS,
    }

    SYMBOLS = {
        "¬": TokenType.NOT,
        "∧": TokenType.AND,
        "∨": TokenType.OR,
        "⟹": TokenType.IMPLIES,
        "→": TokenType.IMPLIES,
        "∀": TokenType.FORALL,
        "∃": TokenType.EXISTS,
        "(": TokenType.LPAREN,
        ")": TokenType.RPAREN,
        "{": TokenType.LBRACE,
        "}": TokenType.RBRACE,
        ",": TokenType.COMMA,
        ".": TokenType.DOT,
        ":": TokenType.COLON,
    }

    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1
        self.column = 1
        self.tokens: List[Token] = []

    def error(self, message: str):
        raise SyntaxError(f"Line {self.line}, column {self.column}: {message}")

    def peek(self, offset=0) -> str:
        pos = self.pos + offset
        if pos < len(self.source):
            return self.source[pos]
        return ""

    def advance(self) -> str:
        if self.pos < len(self.source):
            ch = self.source[self.pos]
            self.pos += 1
            if ch == "\n":
                self.line += 1
                self.column = 1
            else:
                self.column += 1
            return ch
        return ""

    def skip_whitespace(self):
        while self.peek() in " \t\n\r":
            self.advance()

    def skip_comment(self):
        if self.peek() == "#":
            while self.peek() and self.peek() != "\n":
                self.advance()

    def read_identifier(self) -> str:
        start = self.pos
        while self.peek() and (self.peek().isalnum() or self.peek() in "_"):
            self.advance()
        return self.source[start:self.pos]

    def tokenize(self) -> List[Token]:
        while self.pos < len(self.source):
            self.skip_whitespace()

            if self.peek() == "#":
                self.skip_comment()
                continue

            if not self.peek():
                break

            line, column = self.line, self.column

            # Multi-character symbols
            two_char = self.peek() + self.peek(1)
            if two_char in self.SYMBOLS:
                self.advance()
                self.advance()
                self.tokens.append(Token(self.SYMBOLS[two_char], two_char, line, column))
                continue

            # Single-character symbols
            ch = self.peek()
            if ch in self.SYMBOLS:
                self.advance()
                self.tokens.append(Token(self.SYMBOLS[ch], ch, line, column))
                continue

            # Identifiers and keywords
            if ch.isalpha() or ch == "_":
                ident = self.read_identifier()
                token_type = self.KEYWORDS.get(ident.lower(), TokenType.ATOM)
                self.tokens.append(Token(token_type, ident, line, column))
                continue

            self.error(f"Unexpected character: {ch}")

        self.tokens.append(Token(TokenType.EOF, "", self.line, self.column))
        return self.tokens

# ============================================================================
# TIER 4: PARSER (TOKENS → AST)
# ============================================================================

class Parser:
    """Parses tokens into an Abstract Syntax Tree."""

    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.pos = 0

    def error(self, message: str):
        token = self.current_token()
        raise SyntaxError(f"Line {token.line}: {message}")

    def current_token(self) -> Token:
        if self.pos < len(self.tokens):
            return self.tokens[self.pos]
        return self.tokens[-1]  # Return EOF

    def peek(self, offset=0) -> Token:
        pos = self.pos + offset
        if pos < len(self.tokens):
            return self.tokens[pos]
        return self.tokens[-1]

    def advance(self) -> Token:
        token = self.current_token()
        if token.type != TokenType.EOF:
            self.pos += 1
        return token

    def expect(self, token_type: TokenType) -> Token:
        token = self.current_token()
        if token.type != token_type:
            self.error(f"Expected {token_type}, got {token.type}")
        return self.advance()

    def parse_program(self) -> Program:
        """Parse a complete program."""
        line, col = self.current_token().line, self.current_token().column
        lemmas = []

        while self.current_token().type != TokenType.EOF:
            lemmas.append(self.parse_lemma())

        return Program(lemmas, line, col)

    def parse_lemma(self) -> LemmaDefinition:
        """Parse: lemma <name> : <assumptions> → <conclusion> [depends_on ...]"""
        line, col = self.current_token().line, self.current_token().column

        self.expect(TokenType.LEMMA)
        name = self.expect(TokenType.ATOM).value
        self.expect(TokenType.COLON)

        # Parse assumptions
        assumptions = []
        conclusion = None
        dependencies = set()

        # If we see assume, parse explicit assumptions
        if self.current_token().type == TokenType.ASSUME:
            self.advance()
            assumptions.append(self.parse_proposition())
            while self.current_token().type == TokenType.AND:
                self.advance()
                assumptions.append(self.parse_proposition())

        # Parse conclusion (after implies or show)
        if self.current_token().type == TokenType.IMPLIES:
            self.advance()
        elif self.current_token().type == TokenType.SHOW:
            self.advance()

        conclusion = self.parse_proposition()

        # Parse optional dependencies
        if self.current_token().type == TokenType.DEPEND_ON:
            self.advance()
            dependencies.add(self.expect(TokenType.ATOM).value)
            while self.current_token().type == TokenType.COMMA:
                self.advance()
                dependencies.add(self.expect(TokenType.ATOM).value)

        # Expect period or end of lemma
        if self.current_token().type == TokenType.DOT:
            self.advance()

        return LemmaDefinition(name, assumptions, conclusion, dependencies, line, col)

    def parse_proposition(self) -> Proposition:
        """Parse a proposition with operator precedence."""
        return self.parse_implication()

    def parse_implication(self) -> Proposition:
        """Parse implication (lowest precedence)."""
        left = self.parse_disjunction()

        while self.current_token().type == TokenType.IMPLIES:
            token = self.advance()
            right = self.parse_disjunction()
            left = Implication(left, right, token.line, token.column)

        return left

    def parse_disjunction(self) -> Proposition:
        """Parse disjunction."""
        left = self.parse_conjunction()

        while self.current_token().type == TokenType.OR:
            token = self.advance()
            right = self.parse_conjunction()
            left = Disjunction(left, right, token.line, token.column)

        return left

    def parse_conjunction(self) -> Proposition:
        """Parse conjunction."""
        left = self.parse_negation()

        while self.current_token().type == TokenType.AND:
            token = self.advance()
            right = self.parse_negation()
            left = Conjunction(left, right, token.line, token.column)

        return left

    def parse_negation(self) -> Proposition:
        """Parse negation."""
        if self.current_token().type == TokenType.NOT:
            token = self.advance()
            operand = self.parse_negation()
            return Negation(operand, token.line, token.column)

        return self.parse_quantified()

    def parse_quantified(self) -> Proposition:
        """Parse universal/existential quantification."""
        if self.current_token().type in (TokenType.FORALL, TokenType.EXISTS):
            token = self.advance()
            quantifier = "forall" if token.type == TokenType.FORALL else "exists"
            var = self.expect(TokenType.ATOM).value
            self.expect(TokenType.DOT)
            body = self.parse_quantified()
            return Quantified(quantifier, var, body, token.line, token.column)

        return self.parse_primary()

    def parse_primary(self) -> Proposition:
        """Parse primary propositions (atoms and parenthesized expressions)."""
        if self.current_token().type == TokenType.LPAREN:
            self.advance()
            prop = self.parse_proposition()
            self.expect(TokenType.RPAREN)
            return prop

        if self.current_token().type == TokenType.ATOM:
            token = self.advance()
            args = []
            if self.current_token().type == TokenType.LPAREN:
                self.advance()
                if self.current_token().type != TokenType.RPAREN:
                    args.append(self.expect(TokenType.ATOM).value)
                    while self.current_token().type == TokenType.COMMA:
                        self.advance()
                        args.append(self.expect(TokenType.ATOM).value)
                self.expect(TokenType.RPAREN)
            return Atom(token.value, args, token.line, token.column)

        self.error(f"Unexpected token: {self.current_token().type}")

# ============================================================================
# TIER 5: SEMANTIC COMPILATION (AST → ALLOY)
# ============================================================================

class AlloyCodeGenerator:
    """Compiles AST to Alloy specification."""

    def __init__(self, program: Program):
        self.program = program
        self.atom_counter = 0
        self.proposition_defs: Dict[str, Tuple[str, str]] = {}  # name -> (alloy_name, definition)
        self.lemma_defs: Dict[str, str] = {}  # name -> alloy_definition

    def generate_atom_name(self) -> str:
        """Generate a unique name for a proposition variable."""
        self.atom_counter += 1
        return f"p{self.atom_counter}"

    def proposition_to_alloy(self, prop: Proposition) -> Tuple[str, List[str]]:
        """
        Convert a proposition to Alloy code.

        Returns: (alloy_code, list_of_introduced_proposition_names)
        """
        introduced = []

        if isinstance(prop, Atom):
            # Atomic proposition: create an InDomain-like structure
            alloy_name = self.generate_atom_name()
            introduced.append(alloy_name)

            if prop.args:
                args_str = ", ".join(prop.args)
                definition = f"""sig {alloy_name} extends Proposition {{
                    predicate: {prop.name}({args_str})
                }}"""
            else:
                definition = f"sig {alloy_name} extends Proposition {{}}"

            self.proposition_defs[alloy_name] = (alloy_name, definition)
            return alloy_name, introduced

        elif isinstance(prop, Negation):
            operand_name, operand_intro = self.proposition_to_alloy(prop.operand)
            not_name = self.generate_atom_name()
            introduced.extend(operand_intro)
            introduced.append(not_name)
            return f"Not[{operand_name}]", introduced

        elif isinstance(prop, Conjunction):
            left_name, left_intro = self.proposition_to_alloy(prop.left)
            right_name, right_intro = self.proposition_to_alloy(prop.right)
            and_name = self.generate_atom_name()
            introduced.extend(left_intro)
            introduced.extend(right_intro)
            introduced.append(and_name)

            definition = f"""sig {and_name} extends Proposition {{
                left: one Proposition,
                right: one Proposition
            }}
            fact {and_name}_sem {{ {and_name}.left = {left_name} and {and_name}.right = {right_name} }}"""

            self.proposition_defs[and_name] = (and_name, definition)
            return and_name, introduced

        elif isinstance(prop, Disjunction):
            left_name, left_intro = self.proposition_to_alloy(prop.left)
            right_name, right_intro = self.proposition_to_alloy(prop.right)
            or_name = self.generate_atom_name()
            introduced.extend(left_intro)
            introduced.extend(right_intro)
            introduced.append(or_name)

            definition = f"""sig {or_name} extends Proposition {{
                left: one Proposition,
                right: one Proposition
            }}
            fact {or_name}_sem {{ {or_name}.left = {left_name} and {or_name}.right = {right_name} }}"""

            self.proposition_defs[or_name] = (or_name, definition)
            return or_name, introduced

        elif isinstance(prop, Implication):
            ante_name, ante_intro = self.proposition_to_alloy(prop.antecedent)
            cons_name, cons_intro = self.proposition_to_alloy(prop.consequent)
            imp_name = self.generate_atom_name()
            introduced.extend(ante_intro)
            introduced.extend(cons_intro)
            introduced.append(imp_name)

            definition = f"""sig {imp_name} extends Proposition {{
                antecedent: one Proposition,
                consequent: one Proposition
            }}
            fact {imp_name}_sem {{ {imp_name}.antecedent = {ante_name} and {imp_name}.consequent = {cons_name} }}"""

            self.proposition_defs[imp_name] = (imp_name, definition)
            return imp_name, introduced

        else:
            raise ValueError(f"Unknown proposition type: {type(prop)}")

    def generate_alloy(self) -> str:
        """Generate complete Alloy specification."""
        alloy_code = """/**
 * AUTO-GENERATED ALLOY SPECIFICATION
 *
 * Compiled from Freehand Lemma DSL
 * Do not edit manually; regenerate from .lemma source
 */

module FreehandLemmas

open FreehandLemmas as FL

// ============================================================================
// Generated Propositions
// ============================================================================

"""

        # Generate proposition definitions
        for prop_name, (alloy_name, definition) in self.proposition_defs.items():
            alloy_code += f"\n{definition}\n"

        # Generate lemma definitions
        alloy_code += "\n// ============================================================================\n"
        alloy_code += "// Generated Lemmas\n"
        alloy_code += "// ============================================================================\n\n"

        for lemma in self.program.lemmas:
            # Generate conclusions
            conclusion_name, conclusion_intro = self.proposition_to_alloy(lemma.conclusion)

            # Generate assumptions
            assumption_names = []
            for assumption in lemma.assumptions:
                ass_name, ass_intro = self.proposition_to_alloy(assumption)
                assumption_names.append(ass_name)

            # Create Lemma instance
            alloy_code += f"""
sig {lemma.name}_lemma extends Lemma {{}}

fact {lemma.name}_def {{
    {lemma.name}_lemma.conclusion = {conclusion_name}
    {lemma.name}_lemma.assumptions = {{ {', '.join(assumption_names) if assumption_names else ''} }}
}}

"""

        return alloy_code

# ============================================================================
# TIER 6: INTEGRATION AND COMPILATION PIPELINE
# ============================================================================

class LemmaCompiler:
    """End-to-end DSL compiler."""

    def __init__(self, dsl_source: str):
        self.source = dsl_source
        self.tokens: Optional[List[Token]] = None
        self.ast: Optional[Program] = None
        self.alloy_code: Optional[str] = None

    def lex(self) -> List[Token]:
        """Lexical analysis."""
        lexer = Lexer(self.source)
        self.tokens = lexer.tokenize()
        return self.tokens

    def parse(self) -> Program:
        """Syntax analysis."""
        if not self.tokens:
            self.lex()
        parser = Parser(self.tokens)
        self.ast = parser.parse_program()
        return self.ast

    def compile(self) -> str:
        """Semantic analysis and code generation."""
        if not self.ast:
            self.parse()
        generator = AlloyCodeGenerator(self.ast)
        self.alloy_code = generator.generate_alloy()
        return self.alloy_code

    def full_pipeline(self) -> Tuple[Program, str]:
        """Execute full compilation pipeline."""
        self.lex()
        self.parse()
        self.compile()
        return self.ast, self.alloy_code

# ============================================================================
# EXAMPLE USAGE
# ============================================================================

if __name__ == "__main__":
    # Example DSL program
    dsl_program = """
    # Law of Excluded Middle
    lemma ExcludedMiddle:
        show (P or (not P)).

    # Transitivity of Implication
    lemma Transitivity:
        assume (P implies Q) and (Q implies R)
        show (P implies R).

    # Contrapositive
    lemma Contrapositive:
        assume (P implies Q)
        show ((not Q) implies (not P))
        depends_on Transitivity.
    """

    compiler = LemmaCompiler(dsl_program)
    ast, alloy_code = compiler.full_pipeline()

    print("=== PARSED AST ===")
    print(ast)
    print("\n=== GENERATED ALLOY ===")
    print(alloy_code)
