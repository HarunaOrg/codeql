/**
 * @id js/examples/emptyblock
 * @name Empty blocks
 * @description Finds empty block statements
 * @kind problem
 * @tags empty
 *       block
 *       statement
 */

import javascript

from BlockStmt blk
where not exists(blk.getAStmt())
select blk
