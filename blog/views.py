from django.shortcuts import render

from django.views import generic


class PostsIndexView(generic.ArchiveIndexView):
    template_name = "blog/index.html"


class PostDetailView(generic.DetailView):
    template_name = "blog/post.html"
