from flask import Blueprint, request, jsonify, render_template
from .parser import parsed_dict 

# Crea il blueprint invece di usare 'app' direttamente
search_bp = Blueprint('search', __name__)

@search_bp.route('/search', methods=['GET'])
def search_word():
    input_word = request.args.get('q', '').strip()
    search_lang = request.args.get('lang', 'chinese').strip().lower()  # Default a 'chinese'
    
    if not input_word:
        return jsonify({'error': 'Nessun termine di ricerca'}), 400
    
    results = []
    
    for entry in parsed_dict:
        # Ricerca per caratteri ESATTI (cinese)
        char_match = (input_word == entry['simplified'] or 
                     input_word == entry['traditional'])
        
        # Ricerca per pinyin (senza toni e spazi)
        def clean_pinyin(pinyin):
            return (pinyin.lower()
                    .replace(" ", "")
                    .replace("1", "").replace("2", "")
                    .replace("3", "").replace("4", ""))
        
        pinyin_match = False
        if search_lang == 'chinese':
            pinyin_match = (clean_pinyin(input_word) == clean_pinyin(entry['pinyin']))
        
        # Ricerca nelle definizioni (inglese)
        english_match = any(input_word.lower() in d.lower() 
                           for d in entry['english'].split('/'))
        
        if search_lang == 'chinese' and (char_match or pinyin_match):
            results.append({
                'traditional': entry['traditional'],
                'simplified': entry['simplified'],
                'pinyin': entry['pinyin'],
                'definitions': [d.strip() for d in entry['english'].split('/')]
            })
        elif search_lang == 'english' and english_match:
            results.append({
                'traditional': entry['traditional'],
                'simplified': entry['simplified'],
                'pinyin': entry['pinyin'],
                'definitions': [d.strip() for d in entry['english'].split('/')]
            })
    
    return jsonify({
        'search_term': input_word,
        'count': len(results),
        'results': results
    })