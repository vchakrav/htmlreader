Goal: To parse html snippets and extract data from them
The html snippets have annotations in attibutes: rmx-name = "some_name", and rmx-type="type" 
rmx-type can be text, image, or list
if its a list then the goal is to extract a sublist of data from the children, otherwise you return
the an object consisting of key some_name, and the extracted value
for text, the value is the innerText of the node
for image, its the src (url)
