/**
 * @id js/examples/taggedtemplates
 * @name Tagged templates
 * @description Finds tagged template expressions
 * @kind problem
 * @tags template
 *       ECMAScript 6
 *       ECMAScript 2015
 */

import javascript

from TaggedTemplateExpr e
select e.getTag(), e.getTemplate()
