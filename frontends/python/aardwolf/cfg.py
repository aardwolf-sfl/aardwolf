from collections import OrderedDict


class Block:
    def __init__(self, id):
        self.id_ = id
        self.body_ = []
        self.succ_ = []
        self.pred_ = []
        self.exits_ = []
        self.frozen_ = False

    def add_node(self, node):
        self.body_.append(node)

    def add_succ(self, block, force=False):
        assert isinstance(block, Block)
        if not self.frozen_ or force:
            self.succ_.append(block)
            block.pred_.append(self)

    def add_exit(self, block):
        assert isinstance(block, Block)
        if not self.frozen_:
            self.exits_.append(block)

    def freeze(self):
        self.frozen_ = True

    def normalize(self):
        if len(self.body_) == 0:
            for pred in self.pred_:
                for succ in self.succ_:
                    pred.add_succ(succ, force=True)

                    try:
                        pred.succ_.remove(self)
                    except:
                        pass

                    try:
                        succ.pred_.remove(self)
                    except:
                        pass

            return False
        else:
            return True

    def entry(self):
        return self.body_[0]

    def exit(self):
        return self.body_[-1]

    def succ(self):
        return iter(self.succ_)

    def pred(self):
        return iter(self.pred_)

    def exits(self):
        return iter(self.exits_)

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

    def __hash__(self):
        return hash(self.id_)

    def __eq__(self, other):
        return self.id_ == other.id_


class CFGBuilder:
    def __init__(self):
        self.block_id_ = 0
        self.block_ = None

        self.ctx_stack_ = []
        self.ctx_store_ = OrderedDict()
        self.push_ctx('__main__')

        self.loops_ = []

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

    def push_loop(self):
        self.loops_.append(self.block_)

    def pop_loop(self):
        self.loops_.pop()

    def peek_loop(self):
        return self.loops_[-1]

    def block(self):
        return self.block_

    def _get_prefix(self):
        return self.ctx_stack_[0] if len(self.ctx_stack_) == 1 else '::'.join(self.ctx_stack_[1:])
