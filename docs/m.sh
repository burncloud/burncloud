#!/bin/bash

claude -p '读取docs/plan.md，把准备要完成的所有任务，都拆分成原子级任务，按照 docs/task.md 的格式，重写docs/task.md，同时把新建的每一个原子级任务里面的"passes"表记为false'
