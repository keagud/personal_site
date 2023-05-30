FROM  python:3.11-bullseye
WORKDIR /app
COPY . /app
RUN pip install -r requirements.txt && pip install gunicorn
EXPOSE 8000
CMD ["gunicorn",  "personal_site.wsgi:application", "-b", "0.0.0.0:8000"]
