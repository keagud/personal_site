MANAGE := poetry run python manage.py

serve: debug-on deps collect
	$(MANAGE) runserver

shell:
	$(MANAGE) shell

format:
	( prettier ./**/templates/*/*  -u --write  > /dev/null & ) 
	(  isort  . -q  &&  black . -q    ) & 


test:
	$(MANAGE) test 


deps:
	poetry export -f requirements.txt --output requirements.txt


debug-on:
	export DJANGO_DEBUG='TRUE'

debug-off:
	export DJANGO_DEBUG='FALSE'

clean:
	rm -rf static_files || echo "No files to clean"

collect: clean
	$(MANAGE) collectstatic
	
