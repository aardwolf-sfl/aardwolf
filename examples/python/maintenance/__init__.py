__version__ = '0.1.0'

from datetime import datetime


class Task:
    def __init__(self, name, priority):
        self.name = name
        self.priority_ = priority


class Entity:
    def __init__(self, id, standard, priority_threshold, tasks_threshold, waiting_threshold):
        self.id = id
        self.standard_ = standard
        self.priority_threshold = priority_threshold
        self.tasks_threshold = tasks_threshold
        self.waiting_threshold = waiting_threshold

        self.tasks_ = []
        self.waiting_ = 0

    def add_task(self, name, priority):
        if self.standard_ == 'A':
            timestamp = datetime.now().strftime('%y%m%d')
            standardized = f'A_{name}_{timestamp}'
        elif self.standard_ == 'B':
            timestamp = datetime.now().strftime('%Y%m%d')
            standardized = f'B{name}{timestamp}'

        self.tasks_.append(Task(standardized, priority))

    def decrease_priorities(self):
        for task in self.tasks_:
            task.priority_ -= 1

    def wait(self):
        self.waiting_ += 1

    def should_process(self):
        counter = 0

        if self.waiting_ >= self.waiting_threshold:
            return True

        for task in self.tasks_:
            if task.priority_ == 0:
                return True
            elif task.priority_ < self.priority_threshold:
                counter += 1

        return counter >= self.tasks_threshold

    def prioritized_tasks(self):
        if self.waiting_ >= self.waiting_threshold:
            self.waiting_ = 0
            return self.tasks_
        else:
            return filter(lambda task: task.priority_ < self.priority_threshold, self.tasks_)


def process(entities):
    tasks = []

    for entity in entities:
        if entity.should_process():
            tasks.extend([(entity.id, task.name)
                          for task in entity.prioritized_tasks()])
        else:
            entity.wait()

        entity.decrease_priorities()

    return tasks
