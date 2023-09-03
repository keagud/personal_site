# Todos

Taken from: https://xeiaso.net/blog/new-language-blog-backend-2022-03-02

## High level concepts
- [ ] State management (remembering/recalling things persistently)
- [ ] Basic web serving
- [ ] HTML templating
- [ ] Static file serving
- [ ] Input sanitization (making sure that invalid input can't cause JavaScript injections, etc)
- [ ] Sessions (remembering who a user is between visits to a page)


## Action items
- [ ] An abstract "Post" datatype with a title, publication date, a "URL slug" and content in plain text
- [ ] A home page at / that describes yourself to the world
- [ ] A list of blog posts at /blog
- [ ] Individual blog posts at /blog/{date}/{slug}
- [ ] A /contact page explaining how to contact you
- [ ] An admin area at /admin/* that is protected by a username and password
- [ ] An admin form that lets you create new posts
- [ ] An admin form that lets you edit existing posts
- [ ] An admin form that lets you delete posts
- [ ] An admin view that lets you list all posts
- [ ] Use a CSS theme you like (worst case: pick one at random) to make it look nice
- [ ] HTML templates for the base layout and page details



## Starting Steps
- [ ] Serve a static file at / that contains <h1>Hello, world!</h1>
- [ ] Create a SQLite connection and the posts table
- [ ] Insert a post into your database by hand with the sqlite3 console
- [ ] Wire up a /blog/{date}/{slug} route to show that post
- [ ] Wire up /blog to show all the posts in the database
- [ ] Make static pages for / and /contact
- [ ] Choose a templating language and create a base template
- [ ] Edit all your routes to use that base template
- [ ] Create the admin login page and a POST route to receive the HTML form and check the username/password against something in the SQLite database, creating a session for the admin panel
- [ ] Create an external tool that lets you manually set your username and password
- [ ] Create an admin view that shows all posts
- [ ] Create a form that lets you create a new post and its associated POST handler
- [ ] Create a form that lets you edit an existing post and its associated POST handler
- [ ] Use a CSS theme to make it all look nice



## Extra Credit
- [ ] Add an "updated at" date that shows up if the post has been edited
- [ ] Add tags on posts that let users find posts by tag
- [ ] JSONFeed support
- [ ] "Draft" posts which aren't visible on the public blog index or feeds, but can be shared by URL
- [ ] Use CSRF protection on the admin panel
- [ ] Deploy it on a VPS and serve it to the internet (use Caddy to make this easier)
- [ ] Pagination of post lists
