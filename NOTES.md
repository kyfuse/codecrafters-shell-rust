# Notes

I'll write down learnings as I come across them here.

## 10/18/26

I like our current format. It did require 2 changes in separate places and a new test, but it's not too complicated that making one change requires changing a lot of stuff. Writing the tests does take longer, but makes it less likely we will accidentally break something.

## 8/30/26

Starting on "Locate executable files". I think I overengineered the starting stages a bit. It made it harder to want to continue developing this. Also, keeping things all in one file is going to quickly grow out of control. I can see where the YAGNI principle comes from...

And testing has pros and cons. The pros are that code is less buggy / "probably right when submitting" and refactoring is easier, as you can check with tests. The main con is that writing code to support testing, plus the tests themselves, takes time and makes dev slower. Not sure what to think about this yet.
