from collections import OrderedDict


class Block:
    def __init__(self, id):
        self.id_ = id
        self.body_ = []
        self.succ_ = []

    def add_node(self, node):
        self.body_.append(node)

    def add_succ(self, block):
        assert isinstance(block, Block)
        self.succ_.append(block)

    def entry(self):
        return self.body_[0]

    def exit(self):
        return self.body_[-1]

    def __str__(self):
        output = f'block{self.id_}:\n'

        for node in self.body_:
            output += f'    {node}\n'

        output += '  -> ' + \
            ', '.join([f'block{block.id_}' for block in self.succ_]) + '\n'

        return output

    def __iter__(self):
        return iter(self.body_)

    def __len__(self):
        return len(self.body_)


class CFGBuilder:
    def __init__(self):
        self.block_id_ = 0

        self.ctx_stack_ = []
        self.ctx_store_ = OrderedDict()
        self.push_ctx('__main__')

    def push_ctx(self, name):
        self.ctx_stack_.append(name)
        self.ctx_ = self._get_prefix()
        self.ctx_store_[self.ctx_] = []
        return self.new_block()

    def pop_ctx(self):
        self.ctx_stack_.pop()
        self.ctx_ = self._get_prefix()
        self.block_ = self.ctx_store_[self.ctx_][-1]
        return self.block_

    def new_block(self):
        self.block_id_ += 1
        self.block_ = Block(self.block_id_)
        self.ctx_store_[self.ctx_].append(self.block_)
        return self.block_

    def add_node(self, node):
        self.block_.add_node(node)

    def _get_prefix(self):
        return self.ctx_stack_[0] if len(self.ctx_stack_) == 1 else '::'.join(self.ctx_stack_[1:])
