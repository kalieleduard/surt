# Item List files share the approved Revision

An Item List Revision includes its JSON and the bytes of every named file referenced by an item, rather than approving an index whose documents can change independently. This lets the Task Agent author a separate Markdown document per Task while ensuring the Builder receives the content the human approved; editing any document creates a new Revision that needs reapproval. The engine handles generic item files and gives each child only its own files, avoiding Task-specific entities or dynamically named top-level outputs.
