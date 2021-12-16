/**
 * @id js/examples/jsxattribute
 * @name JSX attributes
 * @description Finds JSX attributes named `dangerouslySetInnerHTML`
 * @kind problem
 * @tags JSX
 *       attribute
 */

import javascript

from JSXAttribute a
where a.getName() = "dangerouslySetInnerHTML"
select a
