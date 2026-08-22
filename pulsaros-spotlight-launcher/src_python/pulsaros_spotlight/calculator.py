"""Math expression evaluation provider for PulsarOS Spotlight."""

from __future__ import annotations

import ast
import math
import operator

_MATH_OPERATORS = {
    ast.Add: operator.add,
    ast.Sub: operator.sub,
    ast.Mult: operator.mul,
    ast.Div: operator.truediv,
    ast.FloorDiv: operator.floordiv,
    ast.Mod: operator.mod,
    ast.Pow: operator.pow,
    ast.USub: operator.neg,
    ast.UAdd: operator.pos,
}

_ALLOWED_MATH_FUNCS = {
    "sqrt": math.sqrt,
    "abs": abs,
    "sin": math.sin,
    "cos": math.cos,
    "tan": math.tan,
    "log": math.log,
    "log10": math.log10,
    "pi": math.pi,
    "e": math.e,
}


def _eval_ast(node: ast.AST) -> float | int:
    if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)):
        return node.value
    if isinstance(node, ast.BinOp):
        left = _eval_ast(node.left)
        right = _eval_ast(node.right)
        op_type = type(node.op)
        if op_type in _MATH_OPERATORS:
            return _MATH_OPERATORS[op_type](left, right)
    if isinstance(node, ast.UnaryOp):
        operand = _eval_ast(node.operand)
        op_type = type(node.op)
        if op_type in _MATH_OPERATORS:
            return _MATH_OPERATORS[op_type](operand)
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
        func_name = node.func.id.lower()
        if func_name in _ALLOWED_MATH_FUNCS and callable(_ALLOWED_MATH_FUNCS[func_name]):
            args = [_eval_ast(arg) for arg in node.args]
            return _ALLOWED_MATH_FUNCS[func_name](*args)
    if isinstance(node, ast.Name):
        name = node.id.lower()
        if name in _ALLOWED_MATH_FUNCS and isinstance(_ALLOWED_MATH_FUNCS[name], (int, float)):
            return _ALLOWED_MATH_FUNCS[name]
    raise ValueError("Invalid math node")


class Calculator:
    """Mathematical expression evaluation engine for Spotlight."""

    @staticmethod
    def evaluate(query: str) -> tuple[str, str] | None:
        """Evaluate math query string into (result_value_str, formatted_snippet).
        Returns None if query is not a valid mathematical expression.
        """
        q = query.strip()
        if not q:
            return None

        clean_q = q.replace("×", "*").replace("÷", "/").replace("^", "**")
        has_op = any(op in clean_q for op in ("+", "-", "*", "/", "%", "**")) or any(
            clean_q.lower().startswith(fn) for fn in ("sqrt", "abs", "sin", "cos", "tan", "log")
        )
        if not has_op:
            return None

        try:
            parsed = ast.parse(clean_q, mode="eval")
            val = _eval_ast(parsed.body)
            if isinstance(val, float) and val.is_integer():
                val = int(val)
            val_str = f"{val:,}" if isinstance(val, int) else f"{val:.6g}"
            return str(val_str), f"{q} = {val_str}"
        except Exception:
            return None
