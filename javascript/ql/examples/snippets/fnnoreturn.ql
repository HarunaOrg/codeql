/**
 * @id js/examples/fnnoreturn
 * @name Functions without return statements
 * @description Finds functions that do not contain a return statement
 * @kind problem
 * @tags function
 *       return
 */

import javascript

from Function f
where
  exists(f.getABodyStmt()) and
  not exists(ReturnStmt r | r.getContainer() = f)
select f
