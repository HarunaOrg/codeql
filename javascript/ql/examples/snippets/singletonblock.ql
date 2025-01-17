/**
 * @id js/examples/singletonblock
 * @name Singleton blocks
 * @description Finds block statements containing a single statement
 * @kind problem
 * @tags block
 *       statement
 */

import javascript

from BlockStmt b
where b.getNumStmt() = 1
select b
