/**
 * @id js/examples/callback
 * @name Callbacks
 * @description Finds functions that are passed as arguments to other functions
 * @kind problem
 * @tags function
 *       callback
 *       higher-order
 */

import javascript

from InvokeExpr invk, DataFlow::FunctionNode f
where f.flowsToExpr(invk.getAnArgument())
select invk, f
