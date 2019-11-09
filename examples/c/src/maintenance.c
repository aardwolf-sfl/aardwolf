#include "maintenance.h"

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

void task_init(struct task *self, const char *name, int priority)
{
    self->name = name;
    self->priority = priority;
}

void task_destroy(struct task *self)
{
    free((void*)self->name);
}

void tasks_vector_init(struct tasks_vector *self)
{
    self->tasks = (struct task*)malloc(sizeof(struct task) * MAX_TASKS);
    self->size = 0;
}

void tasks_vector_destroy(struct tasks_vector *self)
{
    tasks_vector_foreach(task, self, task_destroy(task));
    free(self->tasks);
}

void tasks_vector_append(struct tasks_vector *self, struct task task)
{
    self->tasks[self->size++] = task;
}

void tasks_vector_extend(struct tasks_vector *self, struct tasks_vector other)
{
    for (int i = 0; i < other.size; i++) {
        tasks_vector_append(self, other.tasks[i]);
    }
}

struct task * tasks_vector_at(struct tasks_vector *self, int index)
{
    return &self->tasks[index];
}

void entity_init(struct entity *self, int id, Standard standard, int priority_threshold, int tasks_threshold, int waiting_threshold)
{
    self->id = id;
    self->standard = standard;
    self->priority_threshold = priority_threshold;
    self->tasks_threshold = tasks_threshold;
    self->waiting_threshold = waiting_threshold;

    tasks_vector_init(&self->tasks);
    self->waiting = 0;
}

void entity_destroy(struct entity *self)
{
    tasks_vector_destroy(&self->tasks);
}

void entity_add_task(struct entity *self, const char *name, int priority)
{
    time_t now;
    time(&now);
    char timestamp[10];
    char *standardized = (char*)malloc(sizeof(char) * (strlen(name) + 10));

    if (self->standard == A) {
        strftime(timestamp, 10, "%y%m%d", localtime(&now));
        sprintf(standardized, "A_%s_%s", name, timestamp);
    } else if (self->standard == B) {
        strftime(timestamp, 10, "%Y%m%d", localtime(&now));
        sprintf(standardized, "B%s%s", name, timestamp);
    }

    struct task task;
    task_init(&task, standardized, priority);
    tasks_vector_append(&self->tasks, task);
}

void entity_decrease_priorities(struct entity *self)
{
    tasks_vector_foreach(task, &self->tasks, {
        task->priority--;
    });
}

void entity_wait(struct entity *self)
{
    self->waiting++;
}

bool_t entity_should_process(struct entity *self)
{
    int counter = 0;

    if (self->waiting >= self->waiting_threshold) {
        return true;
    }

    tasks_vector_foreach(task, &self->tasks, {
        if (task->priority == 0) {
            return true;
        } else if (task->priority < self->priority_threshold) {
            counter++;
        }
    });

    return counter >= self->tasks_threshold;
}

void entity_prioritized_tasks(struct entity *self, struct tasks_vector *output)
{
    if (self->waiting >= self->waiting_threshold) {
        self->waiting = 0;
        tasks_vector_extend(output, self->tasks);
    } else {
        tasks_vector_foreach(task, &self->tasks, {
            if (task->priority < self->priority_threshold) {
                tasks_vector_append(output, *task);
            }
        });
    }
}

void process(struct entity *entities, int entities_size, char **tasks, int *tasks_size)
{
    struct tasks_vector temp;
    tasks_vector_init(&temp);
    *tasks_size = 0;

    for (int i = 0; i < entities_size; i++) {
        struct entity *entity = &entities[i];
        if (entity_should_process(entity)) {
            entity_prioritized_tasks(entity, &temp);
            tasks_vector_foreach(task, &temp, {
                strcpy(tasks[(*tasks_size)++], task->name);
            });
            temp.size = 0; // Reset
        } else {
            entity_wait(entity);
        }

        entity_decrease_priorities(entity);
    }

    tasks_vector_destroy(&temp);
}