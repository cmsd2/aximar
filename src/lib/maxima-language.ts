import { StreamLanguage, type StreamParser } from "@codemirror/language";
import { LanguageSupport } from "@codemirror/language";

const KEYWORDS = new Set([
  "if", "then", "else", "elseif",
  "for", "while", "unless", "do", "thru", "step", "from", "in",
  "block", "lambda", "local", "return",
  "true", "false", "and", "or", "not",
  "define", "load", "kill", "quit",
]);

const BUILTINS = new Set([
  "integrate", "diff", "solve", "expand", "factor", "simplify",
  "ratsimp", "trigsimp", "trigexpand", "trigreduce",
  "plot2d", "plot3d", "wxplot2d", "wxplot3d",
  "limit", "sum", "product", "taylor", "powerseries",
  "matrix", "determinant", "invert", "transpose", "eigenvalues", "eigenvectors",
  "subst", "ev", "at", "assume", "forget",
  "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
  "sinh", "cosh", "tanh", "asinh", "acosh", "atanh",
  "sqrt", "abs", "log", "exp", "expt",
  "mod", "gcd", "lcm", "floor", "ceiling", "round",
  "print", "display", "grind", "string",
  "map", "apply", "funmake", "makelist", "append",
  "length", "first", "rest", "last", "part",
  "is", "listp", "numberp", "integerp", "floatnump",
]);

interface MaximaState {
  commentDepth: number;
}

const maximaParser: StreamParser<MaximaState> = {
  startState(): MaximaState {
    return { commentDepth: 0 };
  },

  token(stream, state): string | null {
    // Inside a block comment
    if (state.commentDepth > 0) {
      while (!stream.eol()) {
        if (stream.match("*/")) {
          state.commentDepth--;
          if (state.commentDepth === 0) return "blockComment";
        } else if (stream.match("/*")) {
          state.commentDepth++;
        } else {
          stream.next();
        }
      }
      return "blockComment";
    }

    // Start of block comment
    if (stream.match("/*")) {
      state.commentDepth = 1;
      while (!stream.eol()) {
        if (stream.match("*/")) {
          state.commentDepth--;
          if (state.commentDepth === 0) return "blockComment";
        } else if (stream.match("/*")) {
          state.commentDepth++;
        } else {
          stream.next();
        }
      }
      return "blockComment";
    }

    // Strings
    if (stream.match('"')) {
      while (!stream.eol()) {
        const ch = stream.next();
        if (ch === "\\" ) {
          stream.next(); // skip escaped char
        } else if (ch === '"') {
          return "string";
        }
      }
      return "string";
    }

    // Numbers (including floats and scientific notation)
    if (stream.match(/^[0-9]+(\.[0-9]*)?([eEbBdD][+-]?[0-9]+)?/)) {
      return "number";
    }
    if (stream.match(/^\.[0-9]+([eEbBdD][+-]?[0-9]+)?/)) {
      return "number";
    }

    // Multi-char operators
    if (stream.match(":=") || stream.match("::=") || stream.match("::") ||
        stream.match(">=") || stream.match("<=") || stream.match("#") ||
        stream.match("''") || stream.match("'")) {
      return "operator";
    }

    // Single-char operators and terminators
    if (stream.match(/^[+\-*/^=<>!@,.:;$()[\]{}]/)) {
      return "operator";
    }

    // Identifiers, keywords, builtins
    if (stream.match(/^[a-zA-Z_][a-zA-Z_0-9]*/)) {
      const word = stream.current();
      if (KEYWORDS.has(word)) return "keyword";
      if (BUILTINS.has(word)) return "variableName.standard";
      return "variableName";
    }

    // Skip whitespace
    if (stream.match(/^\s+/)) {
      return null;
    }

    // Fallback: consume one character
    stream.next();
    return null;
  },
};

export const maximaStreamLanguage = StreamLanguage.define(maximaParser);
export const maximaLanguage = new LanguageSupport(maximaStreamLanguage);
