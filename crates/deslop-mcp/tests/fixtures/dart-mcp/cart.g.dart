// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'cart.dart';

Cart _$CartFromJson(Map<String, dynamic> json) => Cart(
      id: json['id'] as String,
      label: json['label'] as String,
      quantity: (json['quantity'] as num).toInt(),
      checked: json['checked'] as bool,
    );

Map<String, dynamic> _$CartToJson(Cart instance) => <String, dynamic>{
      'id': instance.id,
      'label': instance.label,
      'quantity': instance.quantity,
      'checked': instance.checked,
    };
