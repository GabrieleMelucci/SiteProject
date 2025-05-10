from flask import Blueprint, request, jsonify
from .parser import parsed_dict
import re
from functools import lru_cache

search_bp = Blueprint('search', __name__)

@lru_cache(maxsize=1000)
def normalize_string(s):
    return re.sub(r'[^a-zA-Z\u4e00-\u9fff]', '', s.lower())

def precompute_distances():
    precomputed = []
    for entry in parsed_dict:
        precomputed.append({
            'original': entry,
            'simplified': entry['simplified'],
            'traditional': entry['traditional'],
            'pinyin': entry['pinyin'],
            'definitions': [d.strip() for d in entry['english'].split('/')],
            'norm_simplified': normalize_string(entry['simplified']),
            'norm_traditional': normalize_string(entry['traditional']),
            'norm_pinyin': normalize_string(entry['pinyin'])
        })
    return precomputed

precomputed_data = precompute_distances()

def quick_ratio(search, target):
    search_len = len(search)
    target_len = len(target)
    
    if not search_len or not target_len:
        return 0
        
    common = 0
    i = j = 0
    
    while i < search_len and j < target_len:
        if search[i] == target[j]:
            common += 1
            i += 1
        j += 1
    
    return 2.0 * common / (search_len + target_len)

@search_bp.route('/search', methods=['GET'])
def search_word():
    
    input_word = request.args.get('q', '').strip()
    search_lang = request.args.get('lang', 'chinese').strip().lower()
    
    if not input_word:
        return jsonify({'error': 'Nessun termine di ricerca'}), 400
    
    norm_input = normalize_string(input_word)
    if not norm_input:
        return jsonify({'error': 'Termine di ricerca non valido'}), 400
    
    results = []
    input_len = len(norm_input)
    
    for entry in precomputed_data:
        best_ratio = 0
        
        if search_lang == 'chinese':
            ratios = [
                quick_ratio(norm_input, entry['norm_simplified']),
                quick_ratio(norm_input, entry['norm_traditional']),
                quick_ratio(norm_input, entry['norm_pinyin'])
            ]
            best_ratio = max(ratios)
        else:
            for definition in entry['definitions']:
                norm_def = normalize_string(definition)
                ratio = quick_ratio(norm_input, norm_def)
                if ratio > best_ratio:
                    best_ratio = ratio
        
        if best_ratio > 0.8:
            results.append({
                'traditional': entry['traditional'],
                'simplified': entry['simplified'],
                'pinyin': entry['pinyin'],
                'definitions': entry['definitions'],
                'match_score': best_ratio
            })
    
    results.sort(key=lambda x: -x['match_score'])
    best_results = results[:15]
    
    for result in best_results:
        result.pop('match_score', None)
    
    return jsonify({
        'search_term': input_word,
        'count': len(best_results),
        'results': best_results
    })