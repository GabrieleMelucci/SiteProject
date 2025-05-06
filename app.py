from flask import Flask, render_template
from src.search import search_bp 

app = Flask(__name__)

app.register_blueprint(search_bp, url_prefix='/api')

@app.route('/')
def home():
    return render_template('sitechinese.html')

@app.route('/search')
def search():
    return render_template('search.html')

if __name__ == '__main__':
    app.run(debug=True, port=5000)