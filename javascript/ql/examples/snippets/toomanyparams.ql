/**
 * @id js/examples/toomanyparams
 * @name Functions with many parameters
 * @description Finds functions with more than ten parameters
 * @kind problem
 * @tags function
 *       parameter
 *       argument
 */

import javascript

from Function f
where f.getNumParameter() > 10
select f
