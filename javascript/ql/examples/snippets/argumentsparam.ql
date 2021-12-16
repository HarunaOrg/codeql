/**
 * @id js/examples/argumentsparam
 * @name Parameters called 'arguments'
 * @description Finds parameters called 'arguments'
 * @kind problem
 * @tags parameter
 *       arguments
 */

import javascript

from SimpleParameter p
where p.getName() = "arguments"
select p
