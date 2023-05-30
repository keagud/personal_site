from django.urls import path

from . import views

urlpatterns = [path("", views.PostsIndexView.as_view(), name="index")]
