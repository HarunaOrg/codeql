/**
 * @id js/examples/call
 * @name Calls to function
 * @description Finds function calls of the form `eval(...)`
 * @kind problem
 * @tags call
 *       function
 *       eval
 */

import javascript

from CallExpr c
where c.getCalleeName() = "eval"
select c
