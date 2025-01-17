/**
 * @id js/examples/newexpr
 * @name New expressions
 * @description Finds new expressions of the form `new RegExp(...)`
 * @kind problem
 * @tags new
 *       constructor
 *       instantiation
 */

import javascript

from NewExpr new
where new.getCalleeName() = "RegExp"
select new
