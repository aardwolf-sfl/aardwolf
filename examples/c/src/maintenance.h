#ifndef AARDWOLF_EXAMPLE_MAINTENANCE_H
#define AARDWOLF_EXAMPLE_MAINTENANCE_H

#define MAX_TASKS 100

typedef unsigned char bool_t;
#define true 1
#define false 0

typedef enum { A, B } Standard;

struct task {
    const char *name;
    int priority;
};

struct tasks_vector {
    struct task *tasks;
    int size;
};

struct entity {
    int id;
    Standard standard;
    int priority_threshold;
    int tasks_threshold;
    int waiting_threshold;

    struct tasks_vector tasks;
    int waiting;
};

void task_init(struct task *self, const char *name, int priority);
void task_destroy(struct task *self);

void tasks_vector_init(struct tasks_vector *self);
void tasks_vector_destroy(struct tasks_vector *self);

void tasks_vector_append(struct tasks_vector *self, struct task task);
void tasks_vector_extend(struct tasks_vector *self, struct tasks_vector other);
struct task * tasks_vector_at(struct tasks_vector *self, int index);
#define tasks_vector_foreach(var, self, body) for (int i = 0; i < (self)->size; i++) { struct task *var = &(self)->tasks[i]; body; }

void entity_init(struct entity *self, int id, Standard standard, int priority_threshold, int tasks_threshold, int waiting_threshold);
void entity_destroy(struct entity *self);

void entity_add_task(struct entity *self, const char *name, int priority);
void entity_decrease_priorities(struct entity *self);
void entity_wait(struct entity *self);
bool_t entity_should_process(struct entity *self);
void entity_prioritized_tasks(struct entity *self, struct tasks_vector *output);

void process(struct entity *entities, int entities_size, char **tasks, int *tasks_size);

#endif // AARDWOLF_EXAMPLE_MAINTENANCE_H
