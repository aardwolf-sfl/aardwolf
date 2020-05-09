import ast
from copy import copy


class Normalizer(ast.NodeTransformer):
    def visit_FunctionDef(self, node):
        self.generic_visit(node)

        # Replace the implicit `return None` by the explicit node. Fixes loops
        # which would not have the alternative successor when the loop
        # terminates.
        if not isinstance(node.body[-1], ast.Return):
            ret_none = ast.Return(value=ast.Constant(value=None, kind=None))

            ret_none.lineno = node.end_lineno + 1
            ret_none.col_offset = node.col_offset + 4
            ret_none.end_lineno = ret_none.lineno
            ret_none.end_col_offset = ret_none.col_offset
            ast.fix_missing_locations(ret_none)

            node.body.append(ret_none)

            return copy(node)

        return node

    # TODO: ModuleDef, append sys.exit(0) (or a more suitable alternative) call
    # at the and of the module (only if there are non-declaration statements)
    # for the same reason as explicit `return None` for functions
